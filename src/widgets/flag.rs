use crate::key::Key;

use crate::widgets::Widget;

pub trait Flag: Send + Sync {
    fn set(&mut self, value: bool);
    fn get(&self) -> Option<bool>;
    fn toggle(&mut self) {
        if let Some(state) = self.get() {
            self.set(!state)
        }
    }
}

pub struct FlagWidget<F: Flag> {
    label: String,
    flag: F,
    hotkey: Option<Key>,
}

impl<F: Flag> FlagWidget<F> {
    pub fn new(label: &str, flag: F, hotkey: Option<Key>) -> Self {
        Self {
            label: hotkey
                .as_ref()
                .map(|hotkey| format!("{label} ({hotkey})"))
                .unwrap_or_else(|| label.to_string()),
            flag,
            hotkey,
        }
    }
}

impl<F: Flag> Widget for FlagWidget<F> {
    fn render(&mut self, ui: &imgui::Ui) {
        if let Some(mut state) = self.flag.get() {
            if ui.checkbox(&self.label, &mut state) {
                self.flag.set(state);
            }
        } else {
            let token = ui.begin_disabled(true);
            ui.checkbox(&self.label, &mut false);
            token.end();
        }
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        if ui.is_any_item_active() {
            return;
        }

        if self.hotkey.map(|k| k.is_pressed(ui)).unwrap_or(false) {
            self.flag.toggle();
        }
    }
}
