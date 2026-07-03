use alloc::string::ToString;
use alloc::vec::Vec;
use log::info;
use ratatuefi::UefiBackend;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Clear};
use uefi::boot::ScopedProtocol;
use uefi::fs::FileSystem;
use uefi::proto::console::text::{Input, Key, Output};
use uefi::proto::device_path::{DevicePath, LoadedImageDevicePath};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{prelude::*, println};

pub struct App<'a> {
    quit: bool,
    input: &'a mut Input,
    filesystem_devices: Vec<DeviceWithFileSystem>,
}

struct DeviceWithFileSystem {
    // path: DevicePath,
    fs: ScopedProtocol<SimpleFileSystem>,
}

impl<'a> App<'a> {
    pub fn new(input: &'a mut Input) -> Self {
        let mut app = Self {
            quit: false,
            input,
            filesystem_devices: Vec::new(),
        };
        app.refresh_filesystems();
        app
    }

    fn refresh_filesystems(&mut self) {
        self.filesystem_devices.clear();
        self.filesystem_devices.extend(
            boot::find_handles::<DevicePath>()
                .unwrap()
                .into_iter()
                .filter_map(|handle| {
                    let fs = boot::open_protocol_exclusive::<SimpleFileSystem>(handle).ok()?;
                    Some(DeviceWithFileSystem { fs })
                }),
        );
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

        let block = Block::bordered();
        // let inner = block.inner(area);
        frame.render_widget(block, area);
        println!();

        for device in &self.filesystem_devices {
            println!("  - {:?}", device.fs);
        }
    }
}
