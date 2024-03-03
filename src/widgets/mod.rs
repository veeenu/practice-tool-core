use std::sync::mpsc::Sender;

pub mod flag;

pub trait Widget: Send + Sync + 'static {
    fn render(&mut self, _ui: &imgui::Ui);

    fn render_closed(&mut self, _ui: &imgui::Ui) {}

    fn interact(&mut self, _ui: &imgui::Ui) {}

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
