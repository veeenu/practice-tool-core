use crate::widgets::Widget;

pub struct LabelWidget {
    label: String,
}

impl LabelWidget {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

impl Widget for LabelWidget {
    fn render(&mut self, ui: &imgui::Ui) {
        ui.text(&self.label);
    }
}
