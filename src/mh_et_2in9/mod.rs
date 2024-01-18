//! A simple Driver for the MH ET Live 2.9" (B/C) E-Ink Display via SPI
//!
//! # Example for the 2.9" E-Ink Display
//!
//!```rust, no_run
//!# use embedded_hal_mock::*;
//!# fn main() -> Result<(), MockError> {
//!use embedded_graphics::{prelude::*, primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder}};
//!use epd_waveshare::{epd2in13bc::*, prelude::*};
//!#
//!# let expectations = [];
//!# let mut spi = spi::Mock::new(&expectations);
//!# let expectations = [];
//!# let cs_pin = pin::Mock::new(&expectations);
//!# let busy_in = pin::Mock::new(&expectations);
//!# let dc = pin::Mock::new(&expectations);
//!# let rst = pin::Mock::new(&expectations);
//!# let mut delay = delay::MockNoop::new();
//!
//!// Setup EPD
//!let mut epd = MhEt2in9::new(&mut spi, cs_pin, busy_in, dc, rst, &mut delay, None)?;
//!
//!// Use display graphics from embedded-graphics
//!// This display is for the black/white/chromatic pixels
//!let mut tricolor_display = Display2in13bc::default();
//!
//!// Use embedded graphics for drawing a black line
//!let _ = Line::new(Point::new(0, 120), Point::new(0, 200))
//!    .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
//!    .draw(&mut tricolor_display);
//!
//!// We use `chromatic` but it will be shown as red/yellow
//!let _ = Line::new(Point::new(15, 120), Point::new(15, 200))
//!    .into_styled(PrimitiveStyle::with_stroke(TriColor::Chromatic, 1))
//!    .draw(&mut tricolor_display);
//!
//!// Display updated frame
//!epd.update_color_frame(
//!    &mut spi,
//!    &mut delay,
//!    &tricolor_display.bw_buffer(),
//!    &tricolor_display.chromatic_buffer()
//!)?;
//!epd.display_frame(&mut spi, &mut delay)?;
//!
//!// Set the EPD to sleep
//!epd.sleep(&mut spi, &mut delay)?;
//!# Ok(())
//!# }
//!```
use embedded_hal::{
    blocking::{delay::*, spi::Write},
    digital::v2::*,
};

use crate::traits::{
    InternalWiAdditions, RefreshLut, WaveshareDisplay, WaveshareThreeColorDisplay,
};
use crate::{
    interface::DisplayInterface,
    type_a::constants::{LUT_FULL_UPDATE, LUT_PARTIAL_UPDATE},
};

/// Width of MhEt2in9 in pixels
pub const WIDTH: u32 = 128;
/// Height of MhEt2in9 in pixels
pub const HEIGHT: u32 = 296;
/// Default background color (white) of MhEt2in9 display
pub const DEFAULT_BACKGROUND_COLOR: TriColor = TriColor::White;

const NUM_DISPLAY_BITS: u32 = WIDTH * HEIGHT / 8;

const IS_BUSY_LOW: bool = false;
const VCOM_DATA_INTERVAL: u8 = 0x07;
const WHITE_BORDER: u8 = 0x70;
const BLACK_BORDER: u8 = 0x30;
const CHROMATIC_BORDER: u8 = 0xb0;

use crate::color::TriColor;

pub(crate) mod command;
use self::command::Command;
use crate::buffer_len;

/// Full size buffer for use with the 2in9b/c EPD
#[cfg(feature = "graphics")]
pub type Display2in9bc = crate::graphics::Display<
    WIDTH,
    HEIGHT,
    false,
    { buffer_len(WIDTH as usize, HEIGHT as usize * 2) },
    TriColor,
>;

/// MhEt2in9 driver
pub struct MhEt2in9<SPI, CS, BUSY, DC, RST, DELAY> {
    interface: DisplayInterface<SPI, CS, BUSY, DC, RST, DELAY, true>,
    color: TriColor,
    /// Refresh LUT
    refresh: RefreshLut,
}

impl<SPI, CS, BUSY, DC, RST, DELAY> InternalWiAdditions<SPI, CS, BUSY, DC, RST, DELAY>
    for MhEt2in9<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayUs<u32>,
{
    fn init(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.interface.reset(delay, 10_000, 10_000);

        self.wait_until_idle(spi, delay)?;
        self.command(spi, Command::SwReset)?;
        self.wait_until_idle(spi, delay)?;

        self.interface.cmd_with_data(
            spi,
            Command::DriverOutputControl,
            &[(HEIGHT - 1) as u8, ((HEIGHT - 1) >> 8) as u8, 0x00],
        )?;

        self.interface
            .cmd_with_data(spi, Command::DataEntryModeSetting, &[0x03])?;

        // set resolution
        self.send_resolution(spi)?;
        self.interface
            .cmd_with_data(spi, Command::BorderWaveformControl, &[0x05])?;
        self.interface
            .cmd_with_data(spi, Command::TemperatureSensorControlRead, &[0x80])?;
        self.interface
            .cmd_with_data(spi, Command::DisplayUpdateControl1, &[0x80, 0x80])?;
        self.wait_until_idle(spi, delay)?;
        self.set_lut(spi, delay, None)?;

        Ok(())
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> WaveshareThreeColorDisplay<SPI, CS, BUSY, DC, RST, DELAY>
    for MhEt2in9<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayUs<u32>,
{
    fn update_color_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        black: &[u8],
        chromatic: &[u8],
    ) -> Result<(), SPI::Error> {
        self.update_achromatic_frame(spi, delay, black)?;
        self.update_chromatic_frame(spi, delay, chromatic)?;
        Ok(())
    }

    /// Update only the black/white data of the display.
    ///
    /// Finish by calling `update_chromatic_frame`.
    fn update_achromatic_frame(
        &mut self,
        spi: &mut SPI,
        _delay: &mut DELAY,
        black: &[u8],
    ) -> Result<(), SPI::Error> {
        self.interface.cmd(spi, Command::WriteRam)?;
        self.interface.data(spi, black)?;
        Ok(())
    }

    /// Update only chromatic data of the display.
    ///
    /// This data takes precedence over the black/white data.
    fn update_chromatic_frame(
        &mut self,
        spi: &mut SPI,
        _delay: &mut DELAY,
        chromatic: &[u8],
    ) -> Result<(), SPI::Error> {
        self.interface.cmd(spi, Command::WriteRamRed)?;
        let mut new: [u8; buffer_len(WIDTH as usize, HEIGHT as usize * 2)] =
            [0; buffer_len(WIDTH as usize, HEIGHT as usize * 2)];
        for i in 0..chromatic.len() {
            new[i] = !chromatic[i]; // need to flip bits
        }
        self.interface.data(spi, &new)?;
        Ok(())
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> WaveshareDisplay<SPI, CS, BUSY, DC, RST, DELAY>
    for MhEt2in9<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayUs<u32>,
{
    type DisplayColor = TriColor;
    fn new(
        spi: &mut SPI,
        cs: CS,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
        delay_us: Option<u32>,
    ) -> Result<Self, SPI::Error> {
        let interface = DisplayInterface::new(cs, busy, dc, rst, delay_us);
        let color = DEFAULT_BACKGROUND_COLOR;

        let mut epd = MhEt2in9 {
            interface,
            color,
            refresh: RefreshLut::Full,
        };

        epd.init(spi, delay)?;

        Ok(epd)
    }

    fn sleep(&mut self, spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::DeepSleepMode, &[0x01])?;
        Ok(())
    }

    fn wake_up(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.init(spi, delay)
    }

    fn set_background_color(&mut self, color: TriColor) {
        self.color = color;
    }

    fn background_color(&self) -> &TriColor {
        &self.color
    }

    fn width(&self) -> u32 {
        WIDTH
    }

    fn height(&self) -> u32 {
        HEIGHT
    }

    fn update_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.update_achromatic_frame(spi, delay, buffer)?;

        let color = self.color.get_byte_value();
        self.interface.cmd(spi, Command::WriteRamRed)?;
        self.interface.data_x_times(spi, color, NUM_DISPLAY_BITS)
    }

    #[allow(unused)]
    fn update_partial_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        Ok(())
    }

    fn display_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.command(spi, Command::MasterActivation)?;
        self.wait_until_idle(spi, delay)?;
        Ok(())
    }

    fn update_and_display_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.update_frame(spi, buffer, delay)?;
        self.display_frame(spi, delay)?;
        Ok(())
    }

    fn clear_frame(&mut self, spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.send_resolution(spi)?;

        let color = DEFAULT_BACKGROUND_COLOR.get_byte_value();

        // Clear the black
        self.interface.cmd(spi, Command::WriteRam)?;
        self.interface.data_x_times(spi, color, NUM_DISPLAY_BITS)?;

        // Clear the chromatic
        self.interface.cmd(spi, Command::WriteRamRed)?;
        self.interface.data_x_times(spi, color, NUM_DISPLAY_BITS)?;
        Ok(())
    }

    fn set_lut(
        &mut self,
        spi: &mut SPI,
        _delay: &mut DELAY,
        refresh_rate: Option<RefreshLut>,
    ) -> Result<(), SPI::Error> {
        if let Some(refresh_lut) = refresh_rate {
            self.refresh = refresh_lut;
        }
        self.interface.cmd(spi, Command::WriteLutRegister)?;
        match self.refresh {
            RefreshLut::Full => self.interface.data(spi, &LUT_FULL_UPDATE)?,
            RefreshLut::Quick => self.interface.data(spi, &LUT_PARTIAL_UPDATE)?,
        };

        Ok(())
    }

    fn wait_until_idle(&mut self, _spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.interface.wait_until_idle(delay, IS_BUSY_LOW);
        Ok(())
    }
}

impl<SPI, CS, BUSY, DC, RST, DELAY> MhEt2in9<SPI, CS, BUSY, DC, RST, DELAY>
where
    SPI: Write<u8>,
    CS: OutputPin,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayUs<u32>,
{
    fn command(&mut self, spi: &mut SPI, command: Command) -> Result<(), SPI::Error> {
        self.interface.cmd(spi, command)
    }

    fn cmd_with_data(
        &mut self,
        spi: &mut SPI,
        command: Command,
        data: &[u8],
    ) -> Result<(), SPI::Error> {
        self.interface.cmd_with_data(spi, command, data)
    }

    fn send_resolution(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        let w = self.width();
        let h = self.height();

        self.interface.cmd_with_data(
            spi,
            Command::SetRamXAddressStartEndPosition,
            &[0x00, ((w - 1) >> 3) as u8],
        )?;

        self.interface.cmd_with_data(
            spi,
            Command::SetRamYAddressStartEndPosition,
            &[0x00, 0x00, (h - 1) as u8, ((h - 1) >> 8) as u8],
        )?;

        self.interface
            .cmd_with_data(spi, Command::SetRamXAddressCounter, &[0x00])?;

        self.interface
            .cmd_with_data(spi, Command::SetRamYAddressCounter, &[0x00, 0x00])?;
        Ok(())
    }

    /// Set the outer border of the display to the chosen color.
    pub fn set_border_color(&mut self, spi: &mut SPI, color: TriColor) -> Result<(), SPI::Error> {
        let border = match color {
            TriColor::Black => BLACK_BORDER,
            TriColor::White => WHITE_BORDER,
            TriColor::Chromatic => CHROMATIC_BORDER,
        };
        self.interface.cmd_with_data(
            spi,
            Command::BorderWaveformControl,
            &[border | VCOM_DATA_INTERVAL],
        )
    }
}
