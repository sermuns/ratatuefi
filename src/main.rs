#![no_main]
#![no_std]

use core::time::Duration;

use log::info;
use ratatui::{prelude::*, widgets::RatatuiMascot};
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
    let area = frame.area();
    frame.render_widget(
        RatatuiMascot::default(),
        area.centered(Constraint::Length(40), Constraint::Length(30)),
    );
}
