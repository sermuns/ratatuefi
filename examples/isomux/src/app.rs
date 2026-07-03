use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use log::info;
use ratatuefi::UefiBackend;
use ratatui::prelude::*;
use ratatui::text::{ToLine, ToText};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Padding, Paragraph, Row, Table, TableState,
};
use uefi::boot::ScopedProtocol;
use uefi::fs::{FileSystem, Path, PathBuf, SEPARATOR, SEPARATOR_STR};
use uefi::proto::console::text::{Input, Key, Output, ScanCode};
use uefi::proto::device_path::{DevicePath, LoadedImageDevicePath};
use uefi::proto::media::file::{Directory, File, FileInfo};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{CStr16, CString16, Event, prelude::*, println};

pub struct App<'a> {
    quit: bool,
    input: &'a mut Input,
    filesystem_devices: Vec<DeviceWithFileSystem>,
    focused_block: FocusedBlock,
}

enum FocusedBlock {
    DevicesTable(TableState),
    Device {
        device_index: usize,
        current_dir: PathBuf,
        dir_table_state: TableState,
    },
}

enum Action {
    MoveDown,
    MoveUp,
    Confirm,
    Cancel,
}

#[derive(Debug)]
struct DeviceWithFileSystem {
    path: ScopedProtocol<DevicePath>,
    fs: FileSystem,
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
                    let path = handle.device_path().unwrap();
                    let fs = boot::open_protocol_exclusive::<SimpleFileSystem>(handle).ok()?;
                    Some(DeviceWithFileSystem {
                        path,
                        fs: FileSystem::new(fs),
                    })
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
                    let device_index = table_state.selected().unwrap();
                    self.focused_block = FocusedBlock::Device {
                        device_index,
                        // root
                        current_dir: PathBuf::from(SEPARATOR_STR),
                        dir_table_state: TableState::default().with_selected(0),
                    };
                }
                _ => (),
            },
            FocusedBlock::Device {
                device_index,
                ref mut current_dir,
                ref mut dir_table_state,
                ..
            } => {
                let device = &mut self.filesystem_devices[device_index];
                match action {
                    Action::MoveUp => dir_table_state.select_previous(),
                    Action::MoveDown => dir_table_state.select_next(),
                    Action::Cancel => {
                        if let Some(parent) = current_dir.parent() {
                            *current_dir = parent;
                        } else {
                            self.focused_block =
                                FocusedBlock::DevicesTable(TableState::new().with_selected(0));
                        }
                    }
                    Action::Confirm if let Some(entry_index) = dir_table_state.selected() => {
                        let entry = device
                            .fs
                            .read_dir(&*current_dir)
                            .unwrap()
                            .nth(entry_index)
                            .unwrap()
                            .unwrap();
                        if !entry.is_directory() {
                            return;
                        }

                        let file_name = entry.file_name();
                        if file_name == cstr16!(".") {
                            return;
                        }

                        if current_dir.to_cstr16() == SEPARATOR_STR {
                            *current_dir = file_name.try_into().unwrap();
                        } else if file_name == cstr16!("..") {
                            if let Some(parent) = current_dir.parent() {
                                *current_dir = parent;
                            } else {
                                *current_dir = SEPARATOR_STR.into();
                            }
                        } else {
                            current_dir.push(file_name);
                        }
                    }
                    _ => (),
                }
            }
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
                let [heading_area, table_area] = area.layout(
                    &Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).spacing(1),
                );

                frame.render_widget(
                    format!(
                        "Found {} devices with filesystems:",
                        self.filesystem_devices.len()
                    ),
                    heading_area,
                );

                let rows = self
                    .filesystem_devices
                    .iter()
                    .map(|device| Row::new(device.path.to_text()));
                let table =
                    Table::new(rows, [Constraint::Fill(1)]).highlight_symbol("-> ".yellow());

                frame.render_stateful_widget(table, table_area, table_state);
            }
            FocusedBlock::Device {
                device_index,
                ref current_dir,
                ref mut dir_table_state,
                ..
            } => {
                let [text_area, table_area] = area.layout(&Layout::vertical([
                    Constraint::Length(5),
                    Constraint::Fill(1),
                ]));

                frame.render_widget(current_dir.to_text(), text_area);

                let device = &mut self.filesystem_devices[device_index];

                let Ok(entries) = device.fs.read_dir(current_dir) else {
                    return;
                };

                let rows = entries.map(|e| {
                    let e = e.unwrap();
                    Row::from_iter([e.file_size().to_string(), e.file_name().to_string()])
                });

                let widths = [Constraint::Length(10), Constraint::Fill(1)];
                let table = Table::new(rows, widths).highlight_symbol("-> ".yellow());
                frame.render_stateful_widget(table, table_area, dir_table_state);
            }
        }
    }
}
