#![no_std]
#![no_main]

extern crate alloc;

use uefi::prelude::*;

mod app;

use crate::app::App;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    system::with_stdout(|output| {
        system::with_stdin(|input| {
            output.reset(false).unwrap();
            let mut app = App::new(input, output);
            app.run();
        })
    });

    Status::SUCCESS
}
