use crate::key::Key;
use crate::widgets::Widget;

pub trait Flag: Send + Sync {
    fn set(&mut self, value: bool);
    fn get(&self) -> Option<bool>;
    fn toggle(&mut self) -> Option<bool> {
        if let Some(state) = self.get() {
            self.set(!state);
        }

        self.get()
    }
}

pub struct FlagWidget<F: Flag> {
    label: String,
    label_true: String,
    label_false: String,
    flag: F,
    hotkey: Option<Key>,
    logs: Vec<String>,
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
            label_true: format!("{label} activated"),
            label_false: format!("{label} deactivated"),
            logs: Vec::new(),
        }
    }

    fn log_state(&mut self, state: bool) {
        self.logs.push(if state { self.label_true.clone() } else { self.label_false.clone() });
    }
}

impl<F: Flag> Widget for FlagWidget<F> {
    fn render(&mut self, ui: &imgui::Ui) {
        if let Some(mut state) = self.flag.get() {
            if ui.checkbox(&self.label, &mut state) {
                self.flag.set(state);
                self.log_state(state);
            }
        } else {
            let token = ui.begin_disabled(true);
            ui.checkbox(&self.label, &mut false);
            token.end();
        }
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        if self.hotkey.map(|k| k.is_pressed(ui)).unwrap_or(false) {
            if let Some(state) = self.flag.toggle() {
                self.log_state(state);
            }
        }
    }

    fn log(&mut self, tx: crossbeam_channel::Sender<String>) {
        self.logs.drain(..).for_each(|log| {
            tx.send(log).ok();
        });
    }
}
