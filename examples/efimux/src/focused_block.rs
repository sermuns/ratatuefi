use alloc::{format, string::ToString};
use const_format::formatcp;
use ratatui::{
    prelude::*,
    text::{ToLine, ToText},
    widgets::{Block, Borders, Row, Table, TableState},
};
use uefi::fs::PathBuf;
use unicode_consts::arrows::{DOWNWARDS_ARROW, LEFTWARDS_ARROW, RIGHTWARDS_ARROW, UPWARDS_ARROW};

use crate::app::{DeviceWithFileSystem, is_efi_file};

pub enum FocusedBlock {
    DevicesTable(TableState),
    Device {
        device_index: usize,
        current_dir: PathBuf,
        dir_table_state: TableState,
    },
}

impl FocusedBlock {
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        filesystem_devices: &mut [DeviceWithFileSystem],
    ) {
        let [content_area, help_area] = area.layout(&Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(2),
        ]));

        const UP_DOWN_HELP: &str = formatcp!("up: k/{UPWARDS_ARROW}, down: j/{DOWNWARDS_ARROW}");
        let help_block = Block::new()
            .borders(Borders::TOP)
            .border_style(Color::DarkGray);
        frame.render_widget(&help_block, help_area);
        let help_area = help_block.inner(help_area);
        match self {
            FocusedBlock::DevicesTable(table_state) => {
                let [heading_area, table_area] = content_area.layout(
                    &Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).spacing(1),
                );

                let num_devices = filesystem_devices.len();
                frame.render_widget(
                    format!(
                        "Found {} device{} with filesystems:",
                        num_devices,
                        if num_devices == 1 { "" } else { "s" }
                    ),
                    heading_area,
                );

                let rows = filesystem_devices
                    .iter()
                    .map(|device| Row::new(device.path.to_text()));
                let table =
                    Table::new(rows, [Constraint::Fill(1)]).highlight_symbol("-> ".yellow());

                frame.render_stateful_widget(table, table_area, table_state);

                frame.render_widget(
                    formatcp!(" enter: ENTER/SPACE/l/{RIGHTWARDS_ARROW}, {UP_DOWN_HELP}, quit: q "),
                    help_area,
                );
            }
            FocusedBlock::Device {
                device_index,
                current_dir,
                dir_table_state,
                ..
            } => {
                let [text_area, table_area] = content_area.layout(
                    &Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).spacing(1),
                );

                frame.render_widget(current_dir.to_text(), text_area);

                let device = &mut filesystem_devices[*device_index];

                let rows = device.fs.read_dir(&*current_dir).unwrap().map(|entry| {
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

                let (is_file, is_efi_file) = dir_table_state
                    .selected()
                    .map(|n| {
                        let entry = device
                            .fs
                            .read_dir(&*current_dir)
                            .unwrap()
                            .nth(n)
                            .unwrap()
                            .unwrap();
                        (entry.is_regular_file(), is_efi_file(entry.file_name()))
                    })
                    .unwrap_or((false, false));

                const STEP_UP_HELP: &str = formatcp!("step up: ESC/BACKSPACE/h/{LEFTWARDS_ARROW}");
                if is_efi_file {
                    frame.render_widget(
                        formatcp!(" boot: ENTER/SPACE/l/{RIGHTWARDS_ARROW}, {STEP_UP_HELP}, {UP_DOWN_HELP} "),
                        help_area,
                    );
                } else if is_file {
                    frame.render_widget(formatcp!("{STEP_UP_HELP}, {UP_DOWN_HELP} "), help_area);
                } else {
                    frame.render_widget(
                        formatcp!(" enter: ENTER/SPACE/l/{RIGHTWARDS_ARROW}, {STEP_UP_HELP}, {UP_DOWN_HELP} "),
                        help_area,
                    );
                }
            }
        }
    }
}
