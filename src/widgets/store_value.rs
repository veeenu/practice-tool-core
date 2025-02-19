use std::fmt::Write;

use crate::key::Key;
use crate::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

pub trait ReadWrite: Send + Sync + 'static {
    fn read(&mut self) -> bool;
    fn write(&mut self);
    fn label(&self) -> &str;
}

pub struct StoreValue<W: ReadWrite> {
    readwrite: W,
    label: String,
    key: Option<Key>,
    logs: Vec<String>,
}

impl<W: ReadWrite> StoreValue<W> {
    pub fn new(write: W, key: Option<Key>) -> Self {
        let label = write.label();
        let label = match key {
            Some(key) => format!("{label} ({key})",),
            None => label.to_string(),
        };

        Self { readwrite: write, label, key, logs: Vec::new() }
    }

    fn log_state(&mut self) {
        self.readwrite.read();
        self.logs.push(format!("{0} triggered", self.readwrite.label()));
    }
}

impl<W: ReadWrite> Widget for StoreValue<W> {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;
        let button_height = BUTTON_HEIGHT;

        let readable = self.readwrite.read();
        let _token = ui.begin_disabled(!readable);

        self.label.clear();
        let label = self.readwrite.label();
        match self.key {
            Some(key) => write!(self.label, "{label} ({key})").ok(),
            None => write!(self.label, "{label}").ok(),
        };

        if ui.button_with_size(&self.label, [button_width, button_height]) {
            self.readwrite.write();
            self.log_state();
        }
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        if self.key.map(|key| key.is_pressed(ui)).unwrap_or(false) {
            self.action();
        }
    }

    fn action(&mut self) {
        self.readwrite.read();
        self.readwrite.write();
        self.log_state();
    }

    fn log(&mut self, tx: crossbeam_channel::Sender<String>) {
        self.logs.drain(..).for_each(|log| {
            tx.send(log).ok();
        });
    }
}
