use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use log::info;
use ratatuefi::UefiBackend;
use ratatui::prelude::*;
use ratatui::text::ToText;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Row, Table, TableState};
use uefi::boot::ScopedProtocol;
use uefi::fs::FileSystem;
use uefi::proto::console::text::{Input, Key, Output, ScanCode};
use uefi::proto::device_path::{DevicePath, LoadedImageDevicePath};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{Event, prelude::*, println};

pub struct App<'a> {
    quit: bool,
    input: &'a mut Input,
    filesystem_devices: Vec<DeviceWithFileSystem>,
    focused_block: FocusedBlock,
}

enum FocusedBlock {
    DevicesTable(TableState),
    Device { index: usize },
}

enum Action {
    MoveDown,
    MoveUp,
    Confirm,
    Cancel,
}

struct DeviceWithFileSystem {
    path: ScopedProtocol<DevicePath>,
    fs: ScopedProtocol<SimpleFileSystem>,
}

impl<'a> App<'a> {
    pub fn new(input: &'a mut Input) -> Self {
        let mut app = Self {
            quit: false,
            input,
            filesystem_devices: Vec::new(),
            focused_block: FocusedBlock::DevicesTable(TableState::new().with_selected(0)),
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
                    let path = handle.device_path().unwrap();
                    Some(DeviceWithFileSystem { fs, path })
                }),
        );
    }

    fn wait_for_and_handle_keystroke(&mut self, keystroke_wait_events: &mut [Event]) {
        boot::wait_for_event(keystroke_wait_events).unwrap();

        let Ok(Some(keystroke)) = self.input.read_key() else {
            return;
        };
        // println!("{:?}", keystroke);

        match keystroke {
            Key::Printable(c) => match c.into() {
                'q' => self.quit = true,
                'j' => self.perform_action(Action::MoveDown),
                'k' => self.perform_action(Action::MoveUp),
                '\r' | ' ' => self.perform_action(Action::Confirm),
                '\u{8}' => self.perform_action(Action::Cancel),
                _ => (),
            },
            Key::Special(ScanCode::DOWN) => self.perform_action(Action::MoveDown),
            Key::Special(ScanCode::UP) => self.perform_action(Action::MoveUp),
            Key::Special(ScanCode::ESCAPE) => self.perform_action(Action::Cancel),
            _ => (),
        }
    }

    fn perform_action(&mut self, action: Action) {
        match self.focused_block {
            FocusedBlock::DevicesTable(ref mut table_state) => match action {
                Action::MoveUp => table_state.select_previous(),
                Action::MoveDown => table_state.select_next(),
                Action::Confirm => {
                    self.focused_block = FocusedBlock::Device {
                        index: table_state.selected().unwrap(),
                    }
                }
                _ => (),
            },
            FocusedBlock::Device { index } => match action {
                Action::Cancel => {
                    self.focused_block =
                        FocusedBlock::DevicesTable(TableState::new().with_selected(0))
                }
                _ => (),
            },
        }
    }

    pub fn run(&mut self, mut terminal: Terminal<UefiBackend>) {
        let mut keystroke_wait_events = [self.input.wait_for_key_event().unwrap()];

        while !self.quit {
            terminal.draw(|frame| self.draw(frame)).unwrap();
            self.wait_for_and_handle_keystroke(&mut keystroke_wait_events);
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let outer_area = Rect {
            height: frame.area().height - 1,
            ..frame.area()
        };

        let block = Block::bordered()
            .title(concat!(" ", env!("CARGO_PKG_NAME"), " "))
            .title_alignment(HorizontalAlignment::Center)
            .padding(Padding::proportional(1))
            .border_style(Style::new().light_yellow());
        let area = block.inner(outer_area);
        frame.render_widget(block, outer_area);

        match self.focused_block {
            FocusedBlock::DevicesTable(ref mut table_state) => {
                let rows = self
                    .filesystem_devices
                    .iter()
                    .map(|device| Row::new(device.path.to_text()));
                let table = Table::new(rows, [Constraint::Fill(1)]).highlight_symbol(" ->");

                frame.render_stateful_widget(table, area, table_state);
            }
            FocusedBlock::Device { index } => frame.render_widget(index.to_string(), area),
        }
    }
}
