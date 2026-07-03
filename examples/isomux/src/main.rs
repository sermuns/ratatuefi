#![no_std]
#![no_main]

extern crate alloc;

use ratatuefi::UefiBackend;
use ratatui::Terminal;
use uefi::prelude::*;

mod app;

use crate::app::App;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    system::with_stdout(|output| {
        system::with_stdin(|input| {
            output.clear().unwrap();
            let mut app = App::new(input);
            let terminal = Terminal::new(UefiBackend::new(output)).unwrap();
            app.run(terminal);
        })
    });

    Status::SUCCESS
}
