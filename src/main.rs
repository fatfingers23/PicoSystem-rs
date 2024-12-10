#![no_std]
#![no_main]

use core::cell::RefCell;
use core::fmt::Write;

use defmt::*;
use display_interface::{DataFormat, DisplayError, WriteOnlyDataCommand};
use display_interface_spi::SPIInterface;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Input, Level, Output, Pull},
    peripherals::PIO0,
    pio::{FifoJoin, Pio, PioPin, ShiftConfig, ShiftDirection, StateMachine},
    spi::{self, Spi},
    Peripheral,
};
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex};
use embassy_time::{Delay, Duration, Instant, Timer};
use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::Text;
use embedded_graphics::{
    // image::{Image, ImageRawLE},
    mono_font::MonoTextStyleBuilder,
};
use mipidsi::{
    models::ST7789,
    options::{Orientation, RefreshOrder, Rotation, TearingEffect},
    Builder,
};
use pio_proc::pio_file;
use tinybmp::Bmp;
// use st7789::{Orientation, ST7789};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});

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
    // let di = Spi9Bit::new(p.PIO0, miso, mosi, display_cs);

    // let mut display = ST7789::new(di, rst, 240, 240);

    let mut display = Builder::new(ST7789, di)
        .display_size(240, 240)
        .refresh_order(RefreshOrder::new(
            mipidsi::options::VerticalRefreshOrder::BottomToTop,
            mipidsi::options::HorizontalRefreshOrder::RightToLeft,
        ))
        .invert_colors(mipidsi::options::ColorInversion::Inverted)
        .reset_pin(rst)
        .orientation(Orientation::new())
        .init(&mut Delay)
        .unwrap();
    //Display demo
    display.set_tearing_effect(TearingEffect::Vertical).unwrap();
    display.clear(Rgb565::BLACK).unwrap();

    let raw_image_data: ImageRawLE<Rgb565> =
        ImageRawLE::new(include_bytes!("../assets/ferris.raw"), 86);
    let bmp_data = include_bytes!("../assets/issac.bmp");
    let bmp_issac: Bmp<Rgb565> = Bmp::from_slice(bmp_data).unwrap();

    // let ferris = Image::new(&raw_image_data, Point::new(5, 8));

    // draw image on black background

    // ferris.draw(&mut display).unwrap();

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

    let mut frames = 0;

    let start = Instant::now();

    let char_style = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(Rgb565::CSS_YELLOW)
        .background_color(Rgb565::BLACK)
        .build();
    let mut buf = heapless::String::<255>::new();
    let mut ferris_location = Point::new(5, 50);
    loop {
        wait_vsync(&mut vsync).await;

        buf.clear();

        let fps = frames as f32 / start.elapsed().as_millis() as f32 * 1000.0;

        core::write!(&mut buf, "fps: {:.1}", fps).unwrap();
        Text::new(&buf, Point::new(0, 15), char_style)
            .draw(&mut display)
            .unwrap();
        frames += 1;

        if right_button.is_pressed() {
            ferris_location.x += 2;
        }

        if left_button.is_pressed() {
            ferris_location.x -= 2;
            // display.clear(Rgb565::BLACK).unwrap();
            // ferris_location =
        }

        if down_button.is_pressed() {
            ferris_location.y += 2;
        }

        if up_button.is_pressed() {
            ferris_location.y -= 2;
        }

        let ferris = Image::new(&bmp_issac, ferris_location);
        ferris.draw(&mut display).unwrap();
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
    // vsync_pin.wait_for_any_edge().await;

    vsync_pin.wait_for_high().await;
    vsync_pin.wait_for_low().await;
}

pub struct Spi9Bit<'l> {
    sm: StateMachine<'l, PIO0, 0>,
}

impl<'l> Spi9Bit<'l> {
    pub fn new(
        pio: impl Peripheral<P = PIO0> + 'l,
        clk: impl PioPin,
        mosi: impl PioPin,
        cs: impl PioPin,
    ) -> Spi9Bit<'l> {
        let Pio {
            mut common,
            mut sm0,
            ..
        } = Pio::new(pio, Irqs);

        let prg = pio_proc::pio_asm!(
            r#"
            .side_set 2
            .wrap_target

            bitloop:
                out pins, 1        side 0x0
                jmp !osre bitloop  side 0x1     ; Fall-through if TXF empties
                nop                side 0x0 [1] ; CSn back porch

            public entry_point:                 ; Must set X,Y to n-2 before starting!
                pull ifempty       side 0x2 [1] ; Block with CSn high (minimum 2 cycles)

            .wrap                               ; Note ifempty to avoid time-of-check race

            "#,
        );
        let program = prg.program;

        let clk = common.make_pio_pin(clk);
        let mosi = common.make_pio_pin(mosi);
        let cs = common.make_pio_pin(cs);

        sm0.set_pin_dirs(embassy_rp::pio::Direction::Out, &[&clk, &mosi, &cs]);

        // let relocated = RelocatedProgram::new(&prg.program);
        let mut cfg = embassy_rp::pio::Config::default();
        let relocated = common.load_program(&program);
        // cs:  side set 0b10
        // clk: side set 0b01
        // fist side_set, lower bit in side_set
        cfg.use_program(&relocated, &[&clk, &cs]);

        cfg.clock_divider = 1u8.into(); // run at full speed
        cfg.set_out_pins(&[&mosi]);
        //  cfg.set_set_pins(&[&mosi]);
        cfg.shift_out = ShiftConfig {
            auto_fill: false,
            direction: ShiftDirection::Left,
            threshold: 9, // 9-bit mode
        };
        cfg.fifo_join = FifoJoin::TxOnly;
        sm0.set_config(&cfg);

        sm0.set_enable(true);

        Self { sm: sm0 }
    }

    #[inline]
    pub fn write_data(&mut self, val: u8) {
        // no need to busy wait
        while self.sm.tx().full() {}
        self.sm.tx().push(0x80000000 | ((val as u32) << 23));
    }

    #[inline]
    pub fn write_command(&mut self, val: u8) {
        while self.sm.tx().full() {}
        self.sm.tx().push((val as u32) << 23);
    }
}

impl<'l> WriteOnlyDataCommand for Spi9Bit<'l> {
    fn send_commands(&mut self, cmd: DataFormat<'_>) -> Result<(), DisplayError> {
        match cmd {
            DataFormat::U8(cmds) => {
                for &c in cmds {
                    self.write_command(c);
                }
            }
            _ => {
                defmt::todo!();
            }
        }
        Ok(())
    }

    fn send_data(&mut self, buf: DataFormat<'_>) -> Result<(), DisplayError> {
        match buf {
            DataFormat::U8(buf) => {
                for &byte in buf {
                    self.write_data(byte);
                }
            }
            DataFormat::U16BEIter(it) => {
                for raw in it {
                    self.write_data((raw >> 8) as u8);
                    self.write_data((raw & 0xff) as u8);
                }
            }
            _ => {
                defmt::todo!();
            }
        }

        Ok(())
    }
}
