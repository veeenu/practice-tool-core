use crossbeam_channel::Sender;

pub mod flag;
pub mod group;
pub mod label;
pub mod nudge_position;
pub mod position;
pub mod radial_menu;
pub mod savefile_manager;
pub mod stats_editor;
pub mod store_value;

pub const BUTTON_WIDTH: f32 = 320.;
pub const BUTTON_HEIGHT: f32 = 0.;

pub fn scaling_factor(ui: &imgui::Ui) -> f32 {
    let width = ui.io().display_size[0];
    if width > 2000. {
        1. + 1. / 3.
    } else if width > 1200. {
        1.
    } else {
        2. / 3.
    }
}

pub trait Widget: Send + Sync {
    fn render(&mut self, _ui: &imgui::Ui);

    fn render_closed(&mut self, _ui: &imgui::Ui) {}

    fn interact(&mut self, _ui: &imgui::Ui) {}

    fn action(&mut self) {}

    fn cursor_down(&mut self) {}

    fn cursor_up(&mut self) {}

    fn want_enter(&mut self) -> bool {
        false
    }

    fn want_exit(&mut self) -> bool {
        false
    }

    fn log(&mut self, _tx: Sender<String>) {}
}
