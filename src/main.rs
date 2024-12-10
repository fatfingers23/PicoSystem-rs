#![no_std]
#![no_main]

use defmt::*;
use display_interface_spi::SPIInterface;
use embassy_executor::Spawner;
use embassy_rp::pio::{
    Common, Config, InterruptHandler, Irq, Pio, PioPin, ShiftDirection, StateMachine,
};
use embassy_rp::{
    gpio::{Input, Level, Output, Pull},
    spi::{self, Spi},
};
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::image::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use peripherals::{Button, ButtonPressEvent};
use pio_proc::pio_file;
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
    back_light.set_high();

    let mut display_cs = p.PIN_5;
    let mut miso = p.PIN_6;
    let mut mosi = p.PIN_7;

    let vsync = p.PIN_8;

    let mut dc = p.PIN_9;
    let mut rst = p.PIN_4;

    //SPI Display setup
    let mut display_config = spi::Config::default();
    display_config.frequency = 8_000_000;
    display_config.phase = spi::Phase::CaptureOnSecondTransition;
    display_config.polarity = spi::Polarity::IdleHigh;
    let spi = Spi::new_blocking_txonly(p.SPI0, &mut miso, &mut mosi, spi::Config::default());

    let dcx = Output::new(dc, Level::Low);
    let rst = Output::new(rst, Level::Low);

    let di = SPIInterface::new(spi, dcx, Output::new(&mut display_cs, Level::High));
    let mut display = ST7789::new(di, rst, 240, 240);

    //Display demo
    display.init(&mut Delay).unwrap();
    display.set_orientation(Orientation::Portrait).unwrap();

    let raw_image_data = ImageRawLE::new(include_bytes!("../assets/ferris.raw"), 86);
    let ferris = Image::new(&raw_image_data, Point::new(5, 8));

    // draw image on black background
    display.clear(Rgb565::BLACK).unwrap();
    ferris.draw(&mut display).unwrap();

    //Drops display for PIO to pick it up
    // drop(display);

    //PIO display driver
    let prg = pio_proc::pio_asm!(
        ".origin 16",
        "set pindirs, 1",
        ".wrap_target",
        "out pins,1 [19]",
        ".wrap",
    );

    let program_with_defines = pio_proc::pio_file!(
        "./pio/screen.pio",
        select_program("screen"), // Optional if only one program in the file
        options(max_program_size = 32)  // Optional, defaults to 32
    );
    let program = program_with_defines.program;

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
    loop {
        Timer::after_millis(100).await;
    }
}
