use alloc::vec::Vec;
use core::mem::MaybeUninit;
use ratatuefi::UefiBackend;
use ratatui::{
    prelude::*,
    text::ToLine,
    widgets::{Block, Clear, Padding, TableState},
};
use uefi::{
    CStr16, Event,
    boot::{LoadImageSource, ScopedProtocol, image_handle, start_image},
    fs::{FileSystem, SEPARATOR_STR},
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

use crate::focused_block::FocusedBlock;

pub struct App<'a> {
    quit: bool,
    quit_confirmation: Option<bool>,
    input: &'a mut Input,
    filesystem_devices: Vec<DeviceWithFileSystem>,
    focused_block: FocusedBlock,
}

pub fn is_efi_file(name: &CStr16) -> bool {
    let slice = name.as_slice();
    slice.ends_with(cstr16!(".efi").as_slice()) || slice.ends_with(cstr16!(".EFI").as_slice())
}

enum Action {
    MoveDown,
    MoveUp,
    MoveRight,
    Confirm,
    Cancel,
    Refresh,
    MoveLeft,
}

#[derive(Debug)]
pub struct DeviceWithFileSystem {
    pub path: ScopedProtocol<DevicePath>,
    pub fs: FileSystem,
}

impl<'a> App<'a> {
    pub fn new(input: &'a mut Input) -> Self {
        let mut app = Self {
            quit: false,
            quit_confirmation: None,
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
                'q' => self.quit_confirmation = Some(false),
                'h' => self.perform_action(Action::MoveLeft),
                'j' => self.perform_action(Action::MoveDown),
                'k' => self.perform_action(Action::MoveUp),
                'l' => self.perform_action(Action::MoveRight),
                'r' => self.perform_action(Action::Refresh),
                '\r' | ' ' => self.perform_action(Action::Confirm),
                '\u{8}' => self.perform_action(Action::Cancel),
                _ => (),
            },
            Key::Special(ScanCode::LEFT) => self.perform_action(Action::MoveLeft),
            Key::Special(ScanCode::DOWN) => self.perform_action(Action::MoveDown),
            Key::Special(ScanCode::UP) => self.perform_action(Action::MoveUp),
            Key::Special(ScanCode::RIGHT) => self.perform_action(Action::MoveRight),
            Key::Special(ScanCode::ESCAPE) => self.perform_action(Action::Cancel),
            _ => (),
        }
    }

    fn perform_action(&mut self, action: Action) {
        if let Some(do_quit) = &mut self.quit_confirmation {
            match action {
                Action::Cancel => self.quit_confirmation = None,
                Action::Confirm if *do_quit => self.quit = true,
                Action::Confirm => self.quit_confirmation = None,
                Action::MoveRight | Action::MoveLeft => *do_quit = !*do_quit,
                _ => (),
            }
            return;
        }

        match self.focused_block {
            FocusedBlock::DevicesTable(ref mut table_state) => match action {
                Action::MoveUp => table_state.select_previous(),
                Action::MoveDown => table_state.select_next(),
                Action::Confirm | Action::MoveRight
                    if let Some(device_index) = table_state.selected() =>
                {
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
                    Action::Cancel | Action::MoveLeft => {
                        if current_dir.to_cstr16() == SEPARATOR_STR {
                            self.focused_block =
                                FocusedBlock::DevicesTable(TableState::new().with_selected(0));
                        } else if let Some(parent) = current_dir.parent() {
                            *current_dir = parent;
                        } else {
                            *current_dir = SEPARATOR_STR.into();
                        }
                    }
                    Action::Confirm | Action::MoveRight
                        if let Some(entry_index) = dir_table_state.selected() =>
                    {
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
            .border_style(Color::LightYellow);
        frame.render_widget(&block, outer_area);

        let area = block.inner(outer_area);
        self.focused_block
            .draw(frame, area, &mut self.filesystem_devices);

        if let Some(do_quit) = self.quit_confirmation {
            let centered_area = area.centered(Constraint::Length(40), Constraint::Length(10));
            frame.render_widget(Clear, centered_area);

            let popup_block = Block::bordered()
                .border_style(Color::Red)
                .title("Do you really want to quit?");
            frame.render_widget(&popup_block, centered_area);

            let choices_area = popup_block
                .inner(centered_area)
                .centered_vertically(Constraint::Length(1));

            let [no_area, yes_area] =
                choices_area.layout(&Layout::horizontal([Constraint::Fill(1); 2]));
            let no = "No".to_line().centered();
            let yes = "Yes".to_line().centered();
            if do_quit {
                frame.render_widget(no.dark_gray(), no_area);
                frame.render_widget(yes, yes_area);
            } else {
                frame.render_widget(no, no_area);
                frame.render_widget(yes.dark_gray(), yes_area);
            }
        }
    }
}
