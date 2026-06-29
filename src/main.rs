#![no_main]
#![no_std]

use core::time::Duration;

use log::info;
use ratatui::{
    prelude::*,
    text::ToLine,
    widgets::{RatatuiLogo, RatatuiMascot},
};
use uefi::prelude::*;

mod backend;

use crate::backend::UefiBackend;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    info!("started app");

    system::with_stdout(|output| {
        output.reset(false).unwrap();
        let backend = UefiBackend::new(output);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(draw).unwrap();
    });

    boot::stall(Duration::MAX);

    Status::SUCCESS
}

fn draw(frame: &mut Frame) {
    let [top_area, bottom_area] = frame.area().layout(&Layout::vertical([
        Constraint::Fill(2),
        Constraint::Fill(1),
    ]));

    frame.render_widget(
        RatatuiMascot::default(),
        top_area.centered_horizontally(Constraint::Length(34)),
    );

    frame.render_widget(
        "finally, bloat-free Ratatui!".to_line().centered(),
        bottom_area,
    );
}
