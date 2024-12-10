#![no_std]
#![no_main]

use core::cell::RefCell;

use defmt::*;
use display_interface_spi::SPIInterface;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Input, Level, Output, Pull},
    spi::{self, Spi},
};
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex};
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::Text;
use mipidsi::{
    models::ST7789,
    options::{Orientation, Rotation},
    Builder,
};
use pio_proc::pio_file;
// use st7789::{Orientation, ST7789};
use {defmt_rtt as _, panic_probe as _};

mod batch;
mod graphics;
mod peripherals;
mod pico_display;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = peripherals::init(Default::default());
    let delay: Duration = Duration::from_millis(1000);

    let mut led_g = p.LED_G;
    let mut led_r = p.LED_R;
    let mut led_b = p.LED_B;

    // Display pins
    let mut back_light = Output::new(p.PIN_12, Level::Low);
    back_light.set_high();

    let mut display_cs = p.PIN_5;
    let mut miso = p.PIN_6;
    let mut mosi = p.PIN_7;

    let mut vsync = Input::new(p.PIN_8, Pull::Down);

    let mut dc = p.PIN_9;
    let mut rst = p.PIN_4;

    //SPI Display setup
    let mut display_config = spi::Config::default();
    display_config.frequency = 8_000_000;
    display_config.phase = spi::Phase::CaptureOnSecondTransition;
    display_config.polarity = spi::Polarity::IdleHigh;

    let other_config = spi::Config::default();

    let spi = Spi::new_blocking_txonly(p.SPI0, &mut miso, &mut mosi, spi::Config::default());
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    let dcx = Output::new(dc, Level::Low);
    let rst = Output::new(rst, Level::Low);

    let display_spi = SpiDeviceWithConfig::new(
        &spi_bus,
        Output::new(display_cs, Level::High),
        display_config,
    );

    let di = SPIInterface::new(display_spi, dcx);
    // let mut display = ST7789::new(di, rst, 240, 240);

    let mut display = Builder::new(ST7789, di)
        .display_size(240, 240)
        .invert_colors(mipidsi::options::ColorInversion::Inverted)
        .reset_pin(rst)
        .orientation(Orientation::new())
        .init(&mut Delay)
        .unwrap();
    //Display demo
    display.clear(Rgb565::BLACK).unwrap();

    // display.clear(Rgb565::BLACK).unwrap();

    // display.set_orientation(Orientation::Portrait).unwrap();

    let raw_image_data: ImageRawLE<Rgb565> =
        ImageRawLE::new(include_bytes!("../assets/ferris.raw"), 86);
    let ferris = Image::new(&raw_image_data, Point::new(5, 8));

    // draw image on black background

    ferris.draw(&mut display).unwrap();

    // let ferris_two = Image::new(&raw_image_data, Point::new(100, 8));
    // ferris_two.draw(&mut display).unwrap();
    let mut y_button = p.Y_BUTTON;
    let x_button = p.X_BUTTON;
    let a_button = p.A_BUTTON;
    let b_button = p.B_BUTTON;
    let down_button = p.DOWN_BUTTON;
    let right_button = p.RIGHT_BUTTON;
    let left_button = p.LEFT_BUTTON;
    let up_button = p.UP_BUTTON;

    led_b.set_high();
    let mut ferris_location = Point::new(5, 8);
    let mut scroll = 1u16; // absolute scroll offset

    let char_w = 10;
    let char_h = 20;
    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    let text = "Hello World ^_^;";
    let mut text_x = 240;
    let mut text_y = 240 / 2;

    loop {
        match vsync.get_level() {
            Level::Low => {
                // Wait for the VSYNC pin to go high (start of VSYNC)
                // while vsync.get_level() == Level::Low {}
                info!("VSYNC pin low");
            }
            Level::High => {
                // Wait for the VSYNC pin to go low (end of VSYNC)
                // while vsync.get_level() == Level::High {}
                info!("VSYNC pin high");
            }
        }
        // wait_vsync(&mut vsync).await;
        display.clear(Rgb565::BLACK).unwrap();
        if right_button.is_pressed() {
            // Clear the previous position of Ferris

            // display.clear(Rgb565::BLACK).unwrap();

            let ferris = Image::new(&raw_image_data, ferris_location);

            ferris.draw(&mut display).unwrap();
            // move_sprite(&mut display, "Hello, World!", ferris_location.x, ferris_location.y);
            info!("Right button pressed");
        }

        if left_button.is_pressed() {
            ferris_location = Point::new(ferris_location.x - 5, ferris_location.y);
            // display.clear(Rgb565::BLACK).unwrap();
            // ferris.draw(&mut display).unwrap();
            info!("Left button pressed");
        }

        if up_button.is_pressed() {
            text_y += char_h;
            info!("Up button pressed");
        }

        if down_button.is_pressed() {
            text_y -= char_h;
            info!("Down button pressed");
        }

        // Draw text
        let right = Text::new(text, Point::new(text_x, text_y), text_style)
            .draw(&mut display)
            .unwrap();
        text_x = if right.x <= 0 { 240 } else { text_x - char_w };
        // Timer::after_millis(100).await;
    }
}

// fn move_sprite(display: &mut impl DrawTarget<Color = Rgb565>, text: &str, x: i32, y: i32) {
//     display.
//     let style = TextStyle::new(Font6x8, Rgb565::WHITE);
//     Text::new(text, Point::new(x, y))
//         .into_styled(style)
//         .draw(display)
//         .unwrap();
// }

async fn wait_vsync(vsync_pin: &mut Input<'_>) {
    // Wait for the VSYNC pin to go low (end of VSYNC)
    vsync_pin.wait_for_any_edge().await;

    // while vsync_pin.is_high() {
    //     Timer::after(Duration::from_micros(1)).await;
    // }

    // // Wait for the VSYNC pin to go high (start of VSYNC)
    // while vsync_pin.is_low() {
    //     Timer::after(Duration::from_micros(1)).await;
    // }
}
