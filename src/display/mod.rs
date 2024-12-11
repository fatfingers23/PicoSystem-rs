// associated re-typing not supported in rust yet
#![allow(clippy::type_complexity)]

//! This crate provides a ST7789 driver to connect to TFT displays.

mod instruction;

use core::iter::once;
// use instruction::Instruction;

use display_interface::AsyncWriteOnlyDataCommand;
use display_interface::DataFormat::{U16BEIter, U8Iter};
use embedded_hal_1::delay::DelayNs;
use embedded_hal_1::digital::OutputPin;
use instruction::Instruction;

mod graphics;

pub mod batch;

///
/// ST7789 driver to connect to TFT displays.
///
pub struct ST7789<DI, RST>
where
    DI: AsyncWriteOnlyDataCommand,
    RST: OutputPin,
{
    // Display interface
    di: DI,
    // Reset pin.
    rst: Option<RST>,

    // Visible size (x, y)
    size_x: u16,
    size_y: u16,
    // Current orientation
    orientation: Orientation,
}

///
/// Display orientation.
///
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Orientation {
    Portrait = 0b0000_0000,         // no inverting
    Landscape = 0b0110_0000,        // invert column and page/column order
    PortraitSwapped = 0b1100_0000,  // invert page and column order
    LandscapeSwapped = 0b1010_0000, // invert page and page/column order
}

impl Default for Orientation {
    fn default() -> Self {
        Self::Portrait
    }
}

///
/// Tearing effect output setting.
///
#[derive(Copy, Clone)]
pub enum TearingEffect {
    /// Disable output.
    Off,
    /// Output vertical blanking information.
    Vertical,
    /// Output horizontal and vertical blanking information.
    HorizontalAndVertical,
}

#[derive(Copy, Clone, Debug)]
pub enum BacklightState {
    On,
    Off,
}

///
/// An error holding its source (pins or SPI)
///
#[derive(Debug)]
pub enum Error<PinE> {
    DisplayError,
    Pin(PinE),
}

impl<DI, RST, PinE> ST7789<DI, RST>
where
    DI: AsyncWriteOnlyDataCommand,
    RST: OutputPin<Error = PinE>,
{
    ///
    /// Creates a new ST7789 driver instance
    ///
    /// # Arguments
    ///
    /// * `di` - a display interface for talking with the display
    /// * `rst` - display hard reset pin
    /// * `bl` - backlight pin
    /// * `size_x` - x axis resolution of the display in pixels
    /// * `size_y` - y axis resolution of the display in pixels
    ///
    pub fn new(di: DI, rst: Option<RST>, size_x: u16, size_y: u16) -> Self {
        Self {
            di,
            rst,
            size_x,
            size_y,
            orientation: Orientation::default(),
        }
    }

    ///
    /// Runs commands to initialize the display
    ///
    /// # Arguments
    ///
    /// * `delay_source` - mutable reference to a delay provider
    ///
    pub async fn init(&mut self, delay_source: &mut impl DelayNs) -> Result<(), Error<PinE>> {
        self.hard_reset(delay_source)?;
        // if let Some(bl) = self.bl.as_mut() {
        //     bl.set_low().map_err(Error::Pin)?;
        //     delay_source.delay_us(10_000);
        //     bl.set_high().map_err(Error::Pin)?;
        // }

        self.write_command(Instruction::SWRESET).await?; // reset display
        delay_source.delay_us(150_000);
        self.write_command(Instruction::SLPOUT).await?; // turn off sleep
        delay_source.delay_us(10_000);
        self.write_command(Instruction::INVOFF).await?; // turn off invert
        self.write_command(Instruction::VSCRDER).await?; // vertical scroll definition
        self.write_data(&[0u8, 0u8, 0x14u8, 0u8, 0u8, 0u8]).await?; // 0 TSA, 320 VSA, 0 BSA
        self.write_command(Instruction::MADCTL).await?; // left -> right, bottom -> top RGB
        self.write_data(&[0b0000_0000]).await?;
        self.write_command(Instruction::COLMOD).await?; // 16bit 65k colors
        self.write_data(&[0b0101_0101]).await?;
        self.write_command(Instruction::INVON).await?; // hack?
        delay_source.delay_us(10_000);
        self.write_command(Instruction::NORON).await?; // turn on display
        delay_source.delay_us(10_000);
        self.write_command(Instruction::DISPON).await?; // turn on display
        delay_source.delay_us(10_000);
        Ok(())
    }

    ///
    /// Performs a hard reset using the RST pin sequence
    ///
    /// # Arguments
    ///
    /// * `delay_source` - mutable reference to a delay provider
    ///
    pub fn hard_reset(&mut self, delay_source: &mut impl DelayNs) -> Result<(), Error<PinE>> {
        if let Some(rst) = self.rst.as_mut() {
            rst.set_high().map_err(Error::Pin)?;
            delay_source.delay_us(10); // ensure the pin change will get registered
            rst.set_low().map_err(Error::Pin)?;
            delay_source.delay_us(10); // ensure the pin change will get registered
            rst.set_high().map_err(Error::Pin)?;
            delay_source.delay_us(10); // ensure the pin change will get registered
        }

        Ok(())
    }

    ///
    /// Returns currently set orientation
    ///
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    ///
    /// Sets display orientation
    ///
    pub async fn set_orientation(&mut self, orientation: Orientation) -> Result<(), Error<PinE>> {
        self.write_command(Instruction::MADCTL).await?;
        self.write_data(&[orientation as u8]).await?;
        self.orientation = orientation;
        Ok(())
    }

    ///
    /// Sets a pixel color at the given coords.
    ///
    /// # Arguments
    ///
    /// * `x` - x coordinate
    /// * `y` - y coordinate
    /// * `color` - the Rgb565 color value
    ///
    pub async fn set_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<(), Error<PinE>> {
        self.set_address_window(x, y, x, y).await?;
        self.write_command(Instruction::RAMWR).await?;
        self.di
            .send_data(U16BEIter(&mut once(color)))
            .await
            .map_err(|_| Error::DisplayError)?;

        Ok(())
    }

    ///
    /// Sets pixel colors in given rectangle bounds.
    ///
    /// # Arguments
    ///
    /// * `sx` - x coordinate start
    /// * `sy` - y coordinate start
    /// * `ex` - x coordinate end
    /// * `ey` - y coordinate end
    /// * `colors` - anything that can provide `IntoIterator<Item = u16>` to iterate over pixel data
    ///
    pub async fn set_pixels<T>(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
        colors: T,
    ) -> Result<(), Error<PinE>>
    where
        T: IntoIterator<Item = u16>,
    {
        self.set_address_window(sx, sy, ex, ey).await?;
        self.write_command(Instruction::RAMWR).await?;
        self.di
            .send_data(U16BEIter(&mut colors.into_iter()))
            .await
            .map_err(|_| Error::DisplayError)
    }

    ///
    /// Sets scroll offset "shifting" the displayed picture
    /// # Arguments
    ///
    /// * `offset` - scroll offset in pixels
    ///
    pub async fn set_scroll_offset(&mut self, offset: u16) -> Result<(), Error<PinE>> {
        self.write_command(Instruction::VSCAD).await?;
        self.write_data(&offset.to_be_bytes()).await
    }

    ///
    /// Release resources allocated to this driver back.
    /// This returns the display interface and the RST pin deconstructing the driver.
    ///
    pub fn release(self) -> (DI, Option<RST>) {
        (self.di, self.rst)
    }

    async fn write_command(&mut self, command: Instruction) -> Result<(), Error<PinE>> {
        self.di
            .send_commands(U8Iter(&mut once(command as u8)))
            .await
            .map_err(|_| Error::DisplayError)?;
        Ok(())
    }

    async fn write_data(&mut self, data: &[u8]) -> Result<(), Error<PinE>> {
        self.di
            .send_data(U8Iter(&mut data.iter().cloned()))
            .await
            .map_err(|_| Error::DisplayError)
    }

    // Sets the address window for the display.
    async fn set_address_window(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), Error<PinE>> {
        self.write_command(Instruction::CASET).await?;
        self.write_data(&sx.to_be_bytes()).await?;
        self.write_data(&ex.to_be_bytes()).await?;
        self.write_command(Instruction::RASET).await?;
        self.write_data(&sy.to_be_bytes()).await?;
        self.write_data(&ey.to_be_bytes()).await
    }

    ///
    /// Configures the tearing effect output.
    ///
    pub async fn set_tearing_effect(
        &mut self,
        tearing_effect: TearingEffect,
    ) -> Result<(), Error<PinE>> {
        match tearing_effect {
            TearingEffect::Off => self.write_command(Instruction::TEOFF).await,
            TearingEffect::Vertical => {
                self.write_command(Instruction::TEON).await?;
                self.write_data(&[0]).await
            }
            TearingEffect::HorizontalAndVertical => {
                self.write_command(Instruction::TEON).await?;
                self.write_data(&[1]).await
            }
        }
    }
}
