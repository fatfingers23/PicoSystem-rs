#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use core::cell::RefCell;
use core::fmt::Write;
use defmt::*;
// use display_interface_spi::SPIInterface;
use mipidsi::interface::SpiInterface;

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Input, Level, Output, Pull},
    peripherals::PIO0,
    pwm::{Pwm, SetDutyCycle},
    spi::{self, Spi},
};
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex};
use embassy_time::{Delay, Duration, Instant};
use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::Text;
use embedded_graphics_framebuf::FrameBuf;
// use mipidsi::interface::SpiInterface;
use mipidsi::{
    models::ST7789,
    options::{Orientation, TearingEffect},
    Builder,
};
use tinybmp::Bmp;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});

mod peripherals;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = peripherals::init(Default::default());
    let delay: Duration = Duration::from_millis(1000);
    info!("Hello, World!");
    let mut led_g = p.LED_G;
    let mut led_r = p.LED_R;
    let mut led_b = p.LED_B;
    led_b.set_high();

    // Display pins
    let mut back_light = p.SCREEN_BACKLIGHT;
    back_light.set_brightness(75);
    back_light.toggle();

    let mut display_cs = p.PIN_5;
    let mut miso = p.PIN_6;
    let mut mosi = p.PIN_7;

    let mut vsync = Input::new(p.PIN_8, Pull::Down);

    let mut dc = p.PIN_9;
    let mut rst = p.PIN_4;

    //SPI Display setup
    let mut display_config = spi::Config::default();
    display_config.frequency = 80_000_000;
    // display_config.frequency = 8_000_000;
    display_config.phase = spi::Phase::CaptureOnSecondTransition;
    display_config.polarity = spi::Polarity::IdleHigh;

    let spi = Spi::new_blocking_txonly(p.SPI0, &mut miso, &mut mosi, spi::Config::default());
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    let dcx = Output::new(dc, Level::Low);
    let rst = Output::new(rst, Level::Low);

    let display_spi = SpiDeviceWithConfig::new(
        &spi_bus,
        Output::new(display_cs, Level::High),
        display_config,
    );

    // let di = SPIInterface::new(display_spi, dcx);

    let mut buffer = [0_u8; 240 * 240];
    let di = SpiInterface::new(display_spi, dcx, &mut buffer);

    let mut display = Builder::new(ST7789, di)
        .display_size(240, 240)
        // .refresh_order(RefreshOrder::new(
        //     mipidsi::options::VerticalRefreshOrder::BottomToTop,
        //     mipidsi::options::HorizontalRefreshOrder::RightToLeft,
        // ))
        .invert_colors(mipidsi::options::ColorInversion::Inverted)
        .reset_pin(rst)
        .orientation(Orientation::new())
        .init(&mut Delay)
        .unwrap();
    //Display demo
    display.set_tearing_effect(TearingEffect::Vertical).unwrap();
    display.clear(Rgb565::BLACK).unwrap();

    let bmp_data = include_bytes!("../assets/issac.bmp");
    let bmp_issac: Bmp<Rgb565> = Bmp::from_slice(bmp_data).unwrap();

    let background_bmp: Bmp<Rgb565> =
        Bmp::from_slice(include_bytes!("../assets/background.bmp")).unwrap();
    let background = Image::new(&background_bmp, Point::new(0, 0));

    let mut issac_sprite = Sprite::new(Point::new(5, 50), bmp_issac);

    let mut y_button = p.Y_BUTTON;
    let x_button = p.X_BUTTON;
    let a_button = p.A_BUTTON;
    let b_button = p.B_BUTTON;
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

    let mut data = [Rgb565::BLACK; 240 * 240];
    let mut fbuf = FrameBuf::new(&mut data, 240, 240);

    let area = Rectangle::new(Point::new(0, 0), fbuf.size());
    loop {
        wait_vsync(&mut vsync).await;

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
            background.draw(&mut fbuf).unwrap();
        }
        sprite_movement = false;

        //Fps counter
        buf.clear();
        let fps = frames as f32 / start.elapsed().as_millis() as f32 * 1000.0;

        core::write!(&mut buf, "fps: {:.1}", fps).unwrap();
        Text::new(&buf, Point::new(0, 15), char_style)
            .draw(&mut fbuf)
            .unwrap();
        frames += 1;

        issac_sprite.move_sprite(issacs_new_pos, &mut fbuf);
        // if sprite_movement {
        let new_data = fbuf.data.iter_mut().map(|c| *c);

        display.fill_contiguous(&area, new_data).unwrap();
        // display.draw_iter(fbuf.into_iter()).unwrap();

        // }

        // display.fill_contiguous(&area, data).unwrap();
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
