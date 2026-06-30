#![no_std]

use core::fmt::Write;

use ratatui_core::{
    backend::{self, Backend, WindowSize},
    buffer,
    layout::{Position, Size},
    style::Color,
};
use thiserror::Error;
use uefi::proto::console::text::Color as UefiColor;

pub struct UefiBackend<'a> {
    output: &'a mut uefi::proto::console::text::Output,
}

impl<'a> UefiBackend<'a> {
    pub fn new(output: &'a mut uefi::proto::console::text::Output) -> Self {
        Self { output }
    }
}

// TODO:
#[derive(Debug, Error)]
pub enum UefiBackendError {}

fn ratatui_fg_color_to_uefi_color(color: Color) -> UefiColor {
    // TODO: very unfinished
    match color {
        Color::Reset => UefiColor::White,
        Color::Black => UefiColor::White,
        Color::Red => UefiColor::Red,
        Color::Green => UefiColor::Green,
        Color::Yellow => UefiColor::Yellow,
        Color::Blue => UefiColor::Blue,
        Color::Magenta => UefiColor::Magenta,
        Color::Cyan => UefiColor::Cyan,
        // Color::Gray => UefiColor::Gray,
        Color::DarkGray => UefiColor::DarkGray,
        Color::LightRed => UefiColor::LightRed,
        Color::LightGreen => UefiColor::LightGreen,
        // Color::LightYellow => UefiColor::LightYellow,
        Color::LightBlue => UefiColor::LightBlue,
        Color::LightMagenta => UefiColor::LightMagenta,
        Color::LightCyan => UefiColor::LightCyan,
        Color::White => UefiColor::White,
        Color::Indexed(231) => UefiColor::White,
        Color::Indexed(252) => UefiColor::LightGray,
        Color::Indexed(236) => UefiColor::DarkGray,
        Color::Indexed(196) => UefiColor::Red,
        Color::Indexed(232) => UefiColor::Blue,
        Color::Indexed(237) => UefiColor::LightGray,
        Color::Indexed(248) => UefiColor::DarkGray,
        other => panic!("encountered unsupported color {:?}", other),
    }
}

impl Backend for UefiBackend<'_> {
    type Error = UefiBackendError;

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn window_size(&mut self) -> Result<backend::WindowSize, Self::Error> {
        Ok(WindowSize {
            columns_rows: self.size().unwrap(),
            pixels: Size::ZERO, // not necessary, I think
        })
    }

    fn size(&self) -> Result<Size, Self::Error> {
        let current_mode = self.output.current_mode().unwrap().unwrap();
        Ok(Size {
            width: current_mode.columns() as u16,
            height: current_mode.rows() as u16,
        })
    }

    fn clear_region(&mut self, _clear_type: backend::ClearType) -> Result<(), Self::Error> {
        todo!()
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        self.output.clear().unwrap();
        Ok(())
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), Self::Error> {
        let pos = position.into();
        self.output
            .set_cursor_position(pos.x as usize, pos.y as usize)
            .unwrap();
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), Self::Error> {
        let (column, row) = self.output.cursor_position();
        Ok((column as u16, row as u16))
    }

    fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
        let (column, row) = self.output.cursor_position();
        Ok((column as u16, row as u16).into())
    }

    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a buffer::Cell)>,
    {
        for (x, y, cell) in content {
            self.set_cursor_position((x, y)).unwrap();

            let foreground = ratatui_fg_color_to_uefi_color(cell.fg);
            // background only allowed to be one of the first 8 colors:
            // https://docs.rs/uefi/latest/uefi/proto/console/text/struct.Output.html#method.set_color
            // https://docs.rs/uefi/latest/uefi/proto/console/text/enum.Color.html
            let background = UefiColor::Blue;
            self.output.set_color(foreground, background).unwrap();

            self.output.write_str(cell.symbol()).unwrap();
        }
        Ok(())
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), Self::Error> {
        self.output
            .set_cursor_position(x as usize, y as usize)
            .unwrap();
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        let _ = self.output.enable_cursor(true);
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        let _ = self.output.enable_cursor(false);
        Ok(())
    }
}
