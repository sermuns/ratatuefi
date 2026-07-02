use ratatuefi::UefiBackend;
use ratatui::prelude::*;
use uefi::prelude::*;
use uefi::proto::console::text::{Input, Key, Output};

pub struct App<'a> {
    quit: bool,
    input: &'a mut Input,
    terminal: Terminal<UefiBackend<'a>>,
}

impl<'a> App<'a> {
    pub fn new(input: &'a mut Input, output: &'a mut Output) -> Self {
        let backend = UefiBackend::new(output);
        let terminal = Terminal::new(backend).unwrap();
        Self {
            quit: false,
            input,
            terminal,
        }
    }

    fn handle_keystroke(&mut self) {
        let Ok(Some(keystroke)) = self.input.read_key() else {
            return;
        };

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
        }
    }
}
