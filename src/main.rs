#![no_std]
#![no_main]

use defmt::*;
use display_interface_spi::SPIInterface;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Level, Output},
    spi::{self, Spi},
};
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::image::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use st7789::{Orientation, ST7789};
use {defmt_rtt as _, panic_probe as _};

mod peripherals;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = peripherals::init(Default::default());
    let delay: Duration = Duration::from_millis(1000);

    let mut led_g = p.LED_G;
    let mut led_r = p.LED_R;
    let mut led_b = p.LED_B;

    // Display pins
    let mut back_light = Output::new(p.PIN_12, Level::Low);

    let display_cs = p.PIN_5;
    let miso = p.PIN_6;
    let mosi = p.PIN_7;

    let _vsync = p.PIN_8;
    let dc = p.PIN_9;
    let rst = p.PIN_4;

    //SPI Display setup
    let mut display_config = spi::Config::default();
    display_config.frequency = 8_000_000;
    display_config.phase = spi::Phase::CaptureOnSecondTransition;
    display_config.polarity = spi::Polarity::IdleHigh;
    let spi = Spi::new_blocking_txonly(p.SPI0, miso, mosi, spi::Config::default());

    let dcx = Output::new(dc, Level::Low);
    let rst = Output::new(rst, Level::Low);
    back_light.set_high();

    let di = SPIInterface::new(spi, dcx, Output::new(display_cs, Level::High));
    let mut display = ST7789::new(di, rst, 240, 240);

    //Display demo
    display.init(&mut Delay).unwrap();
    display.set_orientation(Orientation::Portrait).unwrap();

    let raw_image_data = ImageRawLE::new(include_bytes!("../assets/ferris.raw"), 86);
    let ferris = Image::new(&raw_image_data, Point::new(34, 8));

    // draw image on black background
    display.clear(Rgb565::BLACK).unwrap();
    ferris.draw(&mut display).unwrap();

    loop {
        led_g.set_high();
        Timer::after(delay).await;
        led_g.set_low();
        led_r.set_high();
        Timer::after(delay).await;
        led_r.set_low();
        led_b.set_high();
        Timer::after(delay).await;
        Timer::after_secs(5).await;
    }
}
