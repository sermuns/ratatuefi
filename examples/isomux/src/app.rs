use core::time::Duration;

use alloc::format;
use alloc::string::String;
use ratatuefi::UefiBackend;
use ratatui::prelude::*;
use uefi::prelude::*;
use uefi::proto::console::text::{Input, Key, Output};

pub struct App<'a> {
    quit: bool,
    input: &'a mut Input,
    terminal: Terminal<UefiBackend<'a>>,
    latest_keypress_string: String,
}

impl<'a> App<'a> {
    pub fn new(input: &'a mut Input, output: &'a mut Output) -> Self {
        let backend = UefiBackend::new(output);
        let terminal = Terminal::new(backend).unwrap();
        Self {
            quit: false,
            input,
            terminal,
            latest_keypress_string: String::new(),
        }
    }

    fn handle_keystroke(&mut self) {
        let keystroke = self.input.read_key().unwrap().unwrap();
        self.latest_keypress_string = format!("{:?}", keystroke);

        match keystroke {
            Key::Printable(c) => match c.into() {
                'q' => self.quit = true,
                _ => (),
            },
            _ => (),
        }
    }

    pub fn run(&mut self) {
        let events_to_wait_for = &mut [self.input.wait_for_key_event().unwrap()];

        while !self.quit {
            boot::wait_for_event(events_to_wait_for).unwrap();
            self.handle_keystroke();

            self.terminal
                .draw(|frame| {
                    frame.render_widget(self.latest_keypress_string.as_str(), frame.area());
                })
                .unwrap();
        }

        self.terminal
            .draw(|frame| {
                frame.render_widget("quitting!", frame.area());
            })
            .unwrap();

        boot::stall(Duration::from_secs(5));
    }
}
