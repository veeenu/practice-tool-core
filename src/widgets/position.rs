use crate::key::Key;
use crate::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

pub trait PositionStorage: Send + Sync + 'static {
    fn save(&mut self);
    fn load(&mut self);
    fn display_current(&mut self) -> &str;
    fn display_stored(&mut self) -> &str;
    fn is_valid(&self) -> bool;
}

pub struct Position<P: PositionStorage> {
    storage: P,
    key_read: Option<Key>,
    key_write: Option<Key>,
    label_load: String,
    label_save: String,
    logs: Vec<String>,
}

impl<P: PositionStorage> Position<P> {
    pub fn new(storage: P, key_load: Option<Key>, key_save: Option<Key>) -> Self {
        let label_load =
            key_load.map(|k| format!("Load ({k})")).unwrap_or_else(|| "Load".to_string());
        let label_save =
            key_save.map(|k| format!("Save ({k})")).unwrap_or_else(|| "Save".to_string());

        Self {
            storage,
            key_write: key_load,
            key_read: key_save,
            label_load,
            label_save,
            logs: Vec::new(),
        }
    }

    pub fn save_position(&mut self) {
        self.storage.save();
        self.logs.push(format!("Saved position  {}", self.storage.display_stored()));
    }

    pub fn load_position(&mut self) {
        self.storage.load();
        self.logs.push(format!("Loaded position {}", self.storage.display_stored()));
    }
}

impl<S: PositionStorage> Widget for Position<S> {
    fn render(&mut self, ui: &imgui::Ui) {
        let valid = self.storage.is_valid();

        let button_width = BUTTON_WIDTH * scaling_factor(ui);
        let _token = ui.begin_disabled(!valid);

        if ui.button_with_size(&self.label_load, [button_width * 0.33 - 4., BUTTON_HEIGHT]) {
            self.load_position();
        }

        ui.same_line();

        if ui.button_with_size(&self.label_save, [button_width * 0.67 - 4., BUTTON_HEIGHT]) {
            self.save_position();
        }

        ui.text(self.storage.display_current());
        ui.text(self.storage.display_stored());
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        if self.key_write.map(|k| k.is_pressed(ui)).unwrap_or(false) {
            self.load_position();
        }

        if self.key_read.map(|k| k.is_pressed(ui)).unwrap_or(false) {
            self.save_position();
        }
    }

    fn action(&mut self) {
        self.load_position();
    }

    fn log(&mut self, tx: crossbeam_channel::Sender<String>) {
        self.logs.drain(..).for_each(|log| {
            tx.send(log).ok();
        });
    }
}
