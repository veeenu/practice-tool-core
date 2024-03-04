use crate::key::Key;
use crate::widgets::position::PositionStorage;
use crate::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

pub trait NudgePositionStorage: PositionStorage {
    fn nudge_up(&mut self);
    fn nudge_down(&mut self);
}

pub struct NudgePosition<N: NudgePositionStorage> {
    nudge_position: N,
    key_nudge_up: Option<Key>,
    key_nudge_down: Option<Key>,
    label_nudge_up: String,
    label_nudge_down: String,
}

impl<N: NudgePositionStorage> NudgePosition<N> {
    pub fn new(nudge_position: N, key_nudge_up: Option<Key>, key_nudge_down: Option<Key>) -> Self {
        let label_nudge_up = match key_nudge_up {
            Some(key_nudge_up) => format!("Nudge up ({key_nudge_up})"),
            None => "Nudge up".to_string(),
        };

        let label_nudge_down = match key_nudge_down {
            Some(key_nudge_down) => format!("Nudge down ({key_nudge_down})"),
            None => "Nudge down".to_string(),
        };

        Self { nudge_position, key_nudge_up, key_nudge_down, label_nudge_up, label_nudge_down }
    }
}

impl<N: NudgePositionStorage> Widget for NudgePosition<N> {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;
        let button_height = BUTTON_HEIGHT;

        if ui.button_with_size(&self.label_nudge_up, [button_width * 0.5 - 4., button_height]) {
            self.nudge_position.nudge_up();
        }

        ui.same_line();

        if ui.button_with_size(&self.label_nudge_down, [button_width * 0.5 - 4., button_height]) {
            self.nudge_position.nudge_down();
        }
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        if ui.is_any_item_active() {
            return;
        }

        if self.key_nudge_up.map(|k| k.is_pressed(ui)).unwrap_or(false) {
            self.nudge_position.nudge_up();
        }

        if self.key_nudge_down.map(|k| k.is_pressed(ui)).unwrap_or(false) {
            self.nudge_position.nudge_down();
        }
    }
}
