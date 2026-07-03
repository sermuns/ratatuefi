use alloc::string::ToString;
use log::info;
use ratatuefi::UefiBackend;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Clear};
use uefi::prelude::*;
use uefi::proto::console::text::{Input, Key, Output};
use uefi::proto::device_path::DevicePath;

pub struct App<'a> {
    quit: bool,
    input: &'a mut Input,
}

impl<'a> App<'a> {
    pub fn new(input: &'a mut Input) -> Self {
        Self { quit: false, input }
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

    pub fn run(&mut self, mut terminal: Terminal<UefiBackend>) {
        let events_to_wait_for = &mut [self.input.wait_for_key_event().unwrap()];

        while !self.quit {
            terminal.draw(|frame| self.draw(frame)).unwrap();
            boot::wait_for_event(events_to_wait_for).unwrap();
            self.handle_keystroke();
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = Rect {
            height: frame.area().height - 1,
            ..frame.area()
        };
        frame.render_widget(Clear, area);
        frame.render_widget(Block::bordered(), area);
        let handles = uefi::boot::find_handles::<DevicePath>().unwrap();
        for handle in handles {
            info!("{}", handle.device_path().unwrap());
        }
    }
}
