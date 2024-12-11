#![allow(non_snake_case)]
use cortex_m::delay::Delay;
use defmt::{info, Format};
pub use embassy_rp::peripherals::*;
use embassy_rp::{
    config::Config,
    gpio::{AnyPin, Input, Level, Output, Pull},
    pwm::{ChannelAPin, ChannelBPin, Pwm, SetDutyCycle, Slice},
    usb::Out,
    Peripheral,
};
use embassy_time::{with_deadline, Duration, Instant, Timer};

#[allow(dead_code)]
pub struct Peripherals {
    pub PIN_0: PIN_0,
    pub PIN_1: PIN_1,
    pub PIN_3: PIN_3,
    pub PIN_4: PIN_4,
    pub PIN_5: PIN_5,
    pub PIN_6: PIN_6,
    pub PIN_7: PIN_7,
    pub PIN_8: PIN_8,
    pub PIN_9: PIN_9,

    pub PIN_27: PIN_27,
    pub PIN_28: PIN_28,
    pub PIN_VBUS_DETECT: PIN_2,
    pub CHARGE_STAT: PIN_24,
    pub BAT_SENSE: PIN_26,
    pub PIN_QSPI_SCLK: PIN_QSPI_SCLK,
    pub PIN_QSPI_SS: PIN_QSPI_SS,
    pub PIN_QSPI_SD0: PIN_QSPI_SD0,
    pub PIN_QSPI_SD1: PIN_QSPI_SD1,
    pub PIN_QSPI_SD2: PIN_QSPI_SD2,
    pub PIN_QSPI_SD3: PIN_QSPI_SD3,
    pub UART0: UART0,
    pub UART1: UART1,
    pub SPI0: SPI0,
    pub SPI1: SPI1,
    pub I2C0: I2C0,
    pub I2C1: I2C1,
    pub DMA_CH0: DMA_CH0,
    pub DMA_CH1: DMA_CH1,
    pub DMA_CH2: DMA_CH2,
    pub DMA_CH3: DMA_CH3,
    pub DMA_CH4: DMA_CH4,
    pub DMA_CH5: DMA_CH5,
    pub DMA_CH6: DMA_CH6,
    pub DMA_CH7: DMA_CH7,
    pub DMA_CH8: DMA_CH8,
    pub DMA_CH9: DMA_CH9,
    pub DMA_CH10: DMA_CH10,
    pub DMA_CH11: DMA_CH11,
    pub PWM_SLICE0: PWM_SLICE0,
    pub PWM_SLICE1: PWM_SLICE1,
    pub PWM_SLICE2: PWM_SLICE2,
    pub PWM_SLICE3: PWM_SLICE3,
    pub PWM_SLICE4: PWM_SLICE4,
    pub PWM_SLICE5: PWM_SLICE5,
    pub PWM_SLICE7: PWM_SLICE7,
    pub USB: USB,
    pub RTC: RTC,
    pub FLASH: FLASH,
    pub ADC: ADC,
    pub ADC_TEMP_SENSOR: ADC_TEMP_SENSOR,
    pub CORE1: CORE1,
    pub PIO0: PIO0,
    pub PIO1: PIO1,
    pub WATCHDOG: WATCHDOG,
    pub BOOTSEL: BOOTSEL,
    //PicoSystem specific peripherals
    pub SCREEN_BACKLIGHT: Led<'static>,
    pub LED_G: Output<'static>,
    pub LED_R: Output<'static>,
    pub LED_B: Output<'static>,
    // pub Y_BUTTON: Input<'static>,
    pub Y_BUTTON: Button<'static>,
    pub X_BUTTON: Button<'static>,
    pub A_BUTTON: Button<'static>,
    pub B_BUTTON: Button<'static>,
    pub DOWN_BUTTON: Button<'static>,
    pub RIGHT_BUTTON: Button<'static>,
    pub LEFT_BUTTON: Button<'static>,
    pub UP_BUTTON: Button<'static>,
    pub AUDIO: Output<'static>,
}

pub struct Led<'a> {
    pwm: Pwm<'a>,
    on: bool,
    brightness: u8,
}

impl<'a> Led<'a> {
    pub fn new<T: Slice>(
        slice: impl Peripheral<P = T> + 'a,
        a: impl Peripheral<P = impl ChannelAPin<T>> + 'a,
    ) -> Self {
        let mut c = embassy_rp::pwm::Config::default();
        c.top = 65535;
        let pwm = Pwm::new_output_a(slice, a, c.clone());
        Self {
            pwm,
            on: false,
            brightness: 100,
        }
    }

    /// Toggles the light or on
    /// If it is toggled on it goes back to the previous brightness
    pub fn toggle(&mut self) {
        self.on = !self.on;
        match self.on {
            true => {
                let _ = self.pwm.set_duty_cycle_percent(self.brightness);
            }
            false => {
                let _ = self.pwm.set_duty_cycle_fully_off();
            }
        }
    }

    /// Sets the brightness of the led by percentage
    pub fn set_brightness(&mut self, brightness: u8) {
        self.brightness = brightness;
        let _ = self.pwm.set_duty_cycle_percent(self.brightness);
    }
}

//TODO move this to a new game engine crate?
pub struct Button<'a> {
    input: Input<'a>,
}

impl<'a> Button<'a> {
    pub fn new(pin: AnyPin) -> Self {
        let input = Input::new(pin, Pull::Up);
        Self { input }
    }

    pub fn is_pressed(&self) -> bool {
        self.input.is_low()
    }

    pub async fn debounce(&mut self) -> Level {
        loop {
            let l1 = self.input.get_level();

            self.input.wait_for_any_edge().await;

            // Timer::after(self.debounce).await;
            Timer::after_millis(20).await;

            let l2 = self.input.get_level();
            if l1 != l2 {
                break l2;
            }
        }
    }
}

pub fn init(config: Config) -> Peripherals {
    let p = embassy_rp::init(config);

    Peripherals {
        PIN_0: p.PIN_0,
        PIN_1: p.PIN_1,
        PIN_3: p.PIN_3,
        PIN_4: p.PIN_4,
        PIN_5: p.PIN_5,
        PIN_6: p.PIN_6,
        PIN_7: p.PIN_7,
        PIN_8: p.PIN_8,
        PIN_9: p.PIN_9,

        PIN_27: p.PIN_27,
        PIN_28: p.PIN_28,
        PIN_VBUS_DETECT: p.PIN_2,
        CHARGE_STAT: p.PIN_24,
        BAT_SENSE: p.PIN_26,
        PIN_QSPI_SCLK: p.PIN_QSPI_SCLK,
        PIN_QSPI_SS: p.PIN_QSPI_SS,
        PIN_QSPI_SD0: p.PIN_QSPI_SD0,
        PIN_QSPI_SD1: p.PIN_QSPI_SD1,
        PIN_QSPI_SD2: p.PIN_QSPI_SD2,
        PIN_QSPI_SD3: p.PIN_QSPI_SD3,
        UART0: p.UART0,
        UART1: p.UART1,
        SPI0: p.SPI0,
        SPI1: p.SPI1,
        I2C0: p.I2C0,
        I2C1: p.I2C1,
        DMA_CH0: p.DMA_CH0,
        DMA_CH1: p.DMA_CH1,
        DMA_CH2: p.DMA_CH2,
        DMA_CH3: p.DMA_CH3,
        DMA_CH4: p.DMA_CH4,
        DMA_CH5: p.DMA_CH5,
        DMA_CH6: p.DMA_CH6,
        DMA_CH7: p.DMA_CH7,
        DMA_CH8: p.DMA_CH8,
        DMA_CH9: p.DMA_CH9,
        DMA_CH10: p.DMA_CH10,
        DMA_CH11: p.DMA_CH11,
        PWM_SLICE0: p.PWM_SLICE0,
        PWM_SLICE1: p.PWM_SLICE1,
        PWM_SLICE2: p.PWM_SLICE2,
        PWM_SLICE3: p.PWM_SLICE3,
        PWM_SLICE4: p.PWM_SLICE4,
        PWM_SLICE5: p.PWM_SLICE5,

        PWM_SLICE7: p.PWM_SLICE7,
        USB: p.USB,
        RTC: p.RTC,
        FLASH: p.FLASH,
        ADC: p.ADC,
        ADC_TEMP_SENSOR: p.ADC_TEMP_SENSOR,
        CORE1: p.CORE1,
        PIO0: p.PIO0,
        PIO1: p.PIO1,
        WATCHDOG: p.WATCHDOG,
        BOOTSEL: p.BOOTSEL,
        //PicoSystem specific peripherals
        SCREEN_BACKLIGHT: Led::new(p.PWM_SLICE6, p.PIN_12),
        //TODO explore moving to led with pwm
        LED_G: Output::new(p.PIN_13, Level::Low),
        LED_R: Output::new(p.PIN_14, Level::Low),
        LED_B: Output::new(p.PIN_15, Level::Low),
        Y_BUTTON: Button::new(AnyPin::from(p.PIN_16)),
        X_BUTTON: Button::new(AnyPin::from(p.PIN_17)),
        A_BUTTON: Button::new(AnyPin::from(p.PIN_18)),
        B_BUTTON: Button::new(AnyPin::from(p.PIN_19)),
        DOWN_BUTTON: Button::new(AnyPin::from(p.PIN_20)),
        RIGHT_BUTTON: Button::new(AnyPin::from(p.PIN_21)),
        LEFT_BUTTON: Button::new(AnyPin::from(p.PIN_22)),
        UP_BUTTON: Button::new(AnyPin::from(p.PIN_23)),
        AUDIO: Output::new(p.PIN_11, Level::Low),
    }
}
