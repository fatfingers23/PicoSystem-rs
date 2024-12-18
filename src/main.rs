#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use core::cell::RefCell;
use core::fmt::Write;
use defmt::info;
// use display_interface_spi::asynch::SPIInterface;
use display_interface_spi::SPIInterface;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Input, Level, Output, Pull},
    peripherals::{PIO0, SPI0},
    spi::{self, Spi},
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
// use embedded_hal_async::spi::SpiBus::Spi;
// use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex};
use display::{
    batch::{to_blocks, to_rows, PixelBlock},
    graphics::framebuffer,
    Orientation, ST7789,
};
use embassy_time::{Delay, Instant};
use embedded_graphics::image::Image;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use tinybmp::Bmp;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});
mod display;
mod peripherals;

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 240;

type Spi0Bus = Mutex<NoopRawMutex, Spi<'static, SPI0, spi::Async>>;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = peripherals::init(Default::default());
    // let mut led_g = p.LED_G;
    // let mut led_r = p.LED_R;
    let mut led_b = p.LED_B;
    led_b.set_high();

    // Display pins
    let mut back_light = p.SCREEN_BACKLIGHT;

    let display_cs = p.PIN_5;
    let mut miso = p.PIN_6;
    let mut mosi = p.PIN_7;

    let mut vsync = Input::new(p.PIN_8, Pull::Down);

    let dc = p.PIN_9;
    let rst = p.PIN_4;

    //SPI Display setup

    let mut spi_config = spi::Config::default();
    spi_config.frequency = 125_000_000u32;
    let spi = Spi::new_txonly(p.SPI0, &mut miso, &mut mosi, p.DMA_CH0, spi_config);

    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(spi);

    let dcx = Output::new(dc, Level::Low);
    let rst = Output::new(rst, Level::Low);

    let mut display_config = spi::Config::default();
    // display_config.frequency = 80_000_000;
    display_config.frequency = 62_500_000u32;

    display_config.phase = spi::Phase::CaptureOnSecondTransition;
    display_config.polarity = spi::Polarity::IdleHigh;

    let display_spi = SpiDeviceWithConfig::new(
        &spi_bus,
        Output::new(display_cs, Level::High),
        display_config,
    );

    let di = SPIInterface::new(display_spi, dcx);

    let mut display = ST7789::new(di, Some(rst), 240, 240);

    let _ = display.init(&mut Delay).await;
    let _ = display
        .set_tearing_effect(display::TearingEffect::Vertical)
        .await;
    let _ = display.set_orientation(Orientation::Portrait).await;
    let _ = display.clear_screen(Rgb565::BLACK).await;
    back_light.set_brightness(50);
    back_light.toggle();

    let bmp_data = include_bytes!("../assets/issac.bmp");
    let bmp_issac: Bmp<Rgb565> = Bmp::from_slice(bmp_data).unwrap();

    let background_bmp: Bmp<Rgb565> =
        Bmp::from_slice(include_bytes!("../assets/background.bmp")).unwrap();
    let background = Image::new(&background_bmp, Point::new(0, 0));

    let mut issac_sprite = Sprite::new(Point::new(5, 50), bmp_issac);

    let down_button = p.DOWN_BUTTON;
    let right_button = p.RIGHT_BUTTON;
    let left_button = p.LEFT_BUTTON;
    let up_button = p.UP_BUTTON;

    let mut frames = 0;

    let start = Instant::now();

    let char_style = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(Rgb565::CSS_YELLOW)
        .background_color(Rgb565::BLACK)
        .build();
    let mut buf = heapless::String::<255>::new();
    let mut issacs_new_pos = Point::new(5, 50);
    let mut sprite_movement = true;

    // let test_dma = p.DMA_CH1;

    loop {
        wait_vsync(&mut vsync).await;
        let draw_start = Instant::now();

        // info!("loop");
        if right_button.is_pressed() {
            issacs_new_pos.x += 2;
            sprite_movement = true;
        }

        if left_button.is_pressed() {
            issacs_new_pos.x -= 2;
            sprite_movement = true;
        }

        if down_button.is_pressed() {
            issacs_new_pos.y += 2;
            sprite_movement = true;
        }

        if up_button.is_pressed() {
            issacs_new_pos.y -= 2;
            sprite_movement = true;
        }

        //background
        if sprite_movement {
            background.draw(&mut display).unwrap();
        }
        sprite_movement = false;

        //Fps counter
        buf.clear();
        let fps = frames as f32 / start.elapsed().as_millis() as f32 * 1000.0;

        core::write!(&mut buf, "fps: {:.1}", fps).unwrap();
        Text::new(&buf, Point::new(0, 15), char_style)
            .draw(&mut display)
            .unwrap();
        frames += 1;

        issac_sprite.move_sprite(issacs_new_pos, &mut display);

        info!("Draw: {:?}", draw_start.elapsed().as_millis());
        let shotgun_start = Instant::now();
        let _ = display.shotgun().await;
        // let buffer = framebuffer();
        // spi.blocking_write(buffer);
        // let tx_transfer = unsafe {
        //     // If we don't assign future to a variable, the data register pointer
        //     // is held across an await and makes the future non-Send.
        //     embassy_rp::dma::write(
        //         p.DMA_CH1,
        //         buffer,
        //         spi.inner.regs().dr().as_ptr() as *mut _,
        //         // self.inner.regs().dr().as_ptr() as *mut _,
        //         T::TX_DREQ,
        //     )
        // };
        // tx_transfer.await;

        info!("Shotgun: {:?}", shotgun_start.elapsed().as_millis());
    }
}

async fn wait_vsync(vsync_pin: &mut Input<'_>) {
    vsync_pin.wait_for_high().await;
    vsync_pin.wait_for_low().await;
}

struct Sprite<'a> {
    point: Point,
    size: Option<Size>,
    bmp_sprite: Bmp<'a, Rgb565>,
    //TODO pass in background color?
}

impl<'a> Sprite<'a> {
    fn new(point: Point, bmp_sprite: Bmp<'a, Rgb565>) -> Self {
        Self {
            point,
            size: None,
            bmp_sprite,
        }
    }
    fn draw(&mut self, display: &mut impl DrawTarget<Color = Rgb565>) {
        let sprite_image = Image::new(&self.bmp_sprite, self.point);
        if self.size.is_none() {
            self.size = Some(sprite_image.bounding_box().size.clone());
        }
        // self.size = sprite_image.bounding_box().size.clone();
        let _ = sprite_image.draw(display);
    }

    fn move_sprite(&mut self, new_location: Point, display: &mut impl DrawTarget<Color = Rgb565>) {
        if self.size.is_none() {
            self.draw(display);
            return;
        }

        if new_location == self.point {
            return;
        }

        if let Some(size) = self.size {
            // let erase_old_sprite = Rectangle::new(self.point, size).into_styled(
            //     PrimitiveStyleBuilder::new()
            //         .fill_color(Rgb565::BLACK)
            //         .build(),
            // );
            // let _ = erase_old_sprite.draw(display);
            self.point = new_location;
            self.draw(display);
        }
    }
}
