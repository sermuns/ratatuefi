use alloc::{format, string::ToString, vec::Vec};
use core::mem::MaybeUninit;
use ratatuefi::UefiBackend;
use ratatui::{
    prelude::*,
    text::ToText,
    widgets::{Block, Padding, Row, Table, TableState},
};
use uefi::{
    CStr16, Event,
    boot::{LoadImageSource, ScopedProtocol, image_handle, start_image},
    fs::{FileSystem, PathBuf, SEPARATOR_STR},
    prelude::*,
    proto::{
        BootPolicy,
        console::text::{Input, Key, ScanCode},
        device_path::{
            DevicePath,
            build::{self, DevicePathBuilder},
        },
        media::fs::SimpleFileSystem,
    },
};

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

fn is_efi_file(name: &CStr16) -> bool {
    let slice = name.as_slice();
    slice.ends_with(cstr16!(".efi").as_slice()) || slice.ends_with(cstr16!(".EFI").as_slice())
}

enum Action {
    MoveDown,
    MoveUp,
    Confirm,
    Cancel,
    Refresh,
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

        match keystroke {
            Key::Printable(c) => match c.into() {
                'q' => self.quit = true,
                'j' => self.perform_action(Action::MoveDown),
                'k' => self.perform_action(Action::MoveUp),
                'r' => self.perform_action(Action::Refresh),
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
                Action::Confirm if let Some(device_index) = table_state.selected() => {
                    self.focused_block = FocusedBlock::Device {
                        device_index,
                        // root
                        current_dir: SEPARATOR_STR.into(),
                        dir_table_state: TableState::default().with_selected(0),
                    };
                }
                Action::Refresh => self.refresh_filesystems(),
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
                        if current_dir.to_cstr16() == SEPARATOR_STR {
                            self.focused_block =
                                FocusedBlock::DevicesTable(TableState::new().with_selected(0));
                        } else if let Some(parent) = current_dir.parent() {
                            *current_dir = parent;
                        } else {
                            *current_dir = SEPARATOR_STR.into();
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

                        let file_name = entry.file_name();
                        if entry.is_directory() {
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
                        } else if is_efi_file(file_name) {
                            let file_path = if current_dir.to_cstr16() == SEPARATOR_STR {
                                file_name.into()
                            } else {
                                current_dir.join(file_name)
                            };

                            let mut buf = [MaybeUninit::uninit(); 1024];
                            let mut builder = DevicePathBuilder::with_buf(&mut buf);

                            for node in device.path.node_iter() {
                                builder = builder.push(&node).unwrap();
                            }

                            builder = builder
                                .push(&build::media::FilePath {
                                    path_name: file_path.to_cstr16(),
                                })
                                .unwrap();

                            let image_handle = boot::load_image(
                                image_handle(),
                                LoadImageSource::FromDevicePath {
                                    device_path: builder.finalize().unwrap(),
                                    boot_policy: BootPolicy::ExactMatch,
                                },
                            )
                            .unwrap();

                            start_image(image_handle).unwrap();
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
                let [text_area, table_area] = area.layout(
                    &Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).spacing(1),
                );

                frame.render_widget(current_dir.to_text(), text_area);

                let device = &mut self.filesystem_devices[device_index];

                let rows = device.fs.read_dir(current_dir).unwrap().map(|entry| {
                    let e = entry.unwrap();

                    let (is_efi_str, color) = if is_efi_file(e.file_name()) {
                        ("*", Color::Yellow)
                    } else {
                        ("", Color::Reset)
                    };

                    Row::from_iter([
                        is_efi_str.to_string(),
                        e.file_size().to_string(),
                        e.file_name().to_string(),
                    ])
                    .fg(color)
                });

                let widths = [
                    Constraint::Length(1),
                    Constraint::Length(8),
                    Constraint::Fill(1),
                ];
                let table = Table::new(rows, widths).highlight_symbol("-> ".yellow());
                frame.render_stateful_widget(table, table_area, dir_table_state);
            }
        }
    }
}
