use crate::key::Key;
use crate::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

pub trait Write: Send + Sync + 'static {
    fn write(&mut self);
}

pub struct StoreValue<W: Write> {
    write: W,
    label: String,
    key: Option<Key>,
}

impl<W: Write> StoreValue<W> {
    pub fn new(write: W, label: &str, key: Option<Key>) -> Self {
        let label = match key {
            Some(key) => format!("{label} ({key})"),
            None => label.to_string(),
        };

        Self { write, label, key }
    }
}

impl<W: Write> Widget for StoreValue<W> {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;
        let button_height = BUTTON_HEIGHT;

        if ui.button_with_size(&self.label, [button_width, button_height]) {
            self.write.write();
        }
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        if ui.is_any_item_active() {
            return;
        }

        if self.key.map(|key| key.is_pressed(ui)).unwrap_or(false) {
            self.write.write();
        }
    }
}
