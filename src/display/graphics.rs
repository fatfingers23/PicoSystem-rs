// use crate::ST7789::batch::DrawBatch;
use crate::display::{Error, Orientation, ST7789};

use defmt::info;
use display_interface::AsyncWriteOnlyDataCommand;
use embedded_graphics_core::pixelcolor::Rgb565;
use embedded_graphics_core::prelude::{DrawTarget, IntoStorage, Point, Size};
use embedded_graphics_core::{
    pixelcolor::raw::{RawData, RawU16},
    primitives::Rectangle,
};
use embedded_graphics_core::{prelude::OriginDimensions, Pixel};
use embedded_hal_1::digital::OutputPin;

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 240;

pub static mut FRAMEBUFFER: [u16; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];

pub fn framebuffer() -> &'static mut [u16; WIDTH * HEIGHT] {
    unsafe { &mut FRAMEBUFFER }
}

impl<DI, RST, PinE> ST7789<DI, RST>
where
    DI: AsyncWriteOnlyDataCommand,
    RST: OutputPin<Error = PinE>,
{
    /// Returns the bounding box for the entire framebuffer.
    fn framebuffer_bounding_box(&self) -> Rectangle {
        let size = match self.orientation {
            Orientation::Portrait | Orientation::PortraitSwapped => Size::new(240, 320),
            Orientation::Landscape | Orientation::LandscapeSwapped => Size::new(320, 240),
        };

        Rectangle::new(Point::zero(), size)
    }
}

impl<DI, RST, PinE> DrawTarget for ST7789<DI, RST>
where
    DI: AsyncWriteOnlyDataCommand,
    RST: OutputPin<Error = PinE>,
{
    type Error = Error<PinE>;
    type Color = Rgb565;

    fn draw_iter<T>(&mut self, pixels: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = Pixel<Rgb565>>,
    {
        const M: u32 = WIDTH as u32 - 1;
        let fb = framebuffer();
        for Pixel(coord, color) in pixels.into_iter() {
            if let Ok((x @ 0..=M, y @ 0..=M)) = coord.try_into() {
                let index: u32 = x + y * WIDTH as u32;
                let color = RawU16::from(color).into_inner();
                fb[index as usize] ^= color.to_be();
            }
        }

        Ok(())
    }
    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let clipped_area = area.intersection(&self.framebuffer_bounding_box().intersection(area));
        if area.bottom_right().is_none() || clipped_area.bottom_right().is_none() {
            return Ok(());
        }

        let skip_top_left = clipped_area.top_left - area.top_left;
        let skip_bottom_right = area.bottom_right().unwrap() - clipped_area.bottom_right().unwrap();

        let fb = framebuffer();
        let mut colors = colors.into_iter();

        for _ in 0..skip_top_left.y {
            for _ in 0..area.size.width {
                colors.next();
            }
        }

        for y in 0..clipped_area.size.height as i32 {
            for _ in 0..skip_top_left.x {
                colors.next();
            }

            let mut index = clipped_area.top_left.x + (clipped_area.top_left.y + y) * WIDTH as i32;
            for _ in 0..clipped_area.size.width {
                let color = colors.next().unwrap();
                let color = RawU16::from(color).into_inner();

                fb[index as usize] = color.to_be();
                index += 1;
            }

            for _ in 0..skip_bottom_right.x {
                colors.next();
            }
        }

        Ok(())
    }

    // fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
    //     let color = RawU16::from(color).into_inner().to_be();
    //     for _ in 0..2 {
    //         let colors = core::iter::repeat(RawU16::from(color).into_inner()).take(WIDTH * HEIGHT);
    //         let _ = self
    //             .set_pixels(0, 0, (WIDTH - 1) as u16, (HEIGHT - 1) as u16, colors)
    //             .await;
    //     }

    //     Ok(())
    // }
}

impl<DI, RST, PinE> OriginDimensions for ST7789<DI, RST>
where
    DI: AsyncWriteOnlyDataCommand,
    RST: OutputPin<Error = PinE>,
{
    fn size(&self) -> Size {
        Size::new(self.size_x.into(), self.size_y.into()) // visible area, not RAM-pixel size
    }
}
