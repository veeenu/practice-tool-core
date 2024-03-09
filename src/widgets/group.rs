use crossbeam_channel::Sender;
use imgui::sys::{igGetCursorPosX, igGetCursorPosY, igGetWindowPos, igSetNextWindowPos, ImVec2};
use imgui::Condition;

use super::{Widget, BUTTON_HEIGHT, BUTTON_WIDTH};
use crate::key::Key;

pub struct Group {
    label: String,
    label_close: String,
    tag: String,
    key_close: Key,
    children: Vec<Box<dyn Widget>>,
}

impl Group {
    pub fn new(label: &str, key_close: Key, commands: Vec<Box<dyn Widget>>) -> Self {
        Self {
            label: label.to_string(),
            tag: format!("##group-{label}"),
            label_close: format!("Close ({key_close})"),
            key_close,
            children: commands,
        }
    }
}

impl Widget for Group {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = super::scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;

        let (x, y) = unsafe {
            let mut wnd_pos = ImVec2::default();
            igGetWindowPos(&mut wnd_pos);
            (igGetCursorPosX() + wnd_pos.x, igGetCursorPosY() + wnd_pos.y)
        };

        if ui.button_with_size(&self.label, [button_width, BUTTON_HEIGHT]) {
            ui.open_popup(&self.tag);
        }

        unsafe {
            igSetNextWindowPos(
                ImVec2::new(x + 200. * scale, y),
                Condition::Always as i8 as _,
                ImVec2::new(0., 0.),
            )
        };

        if let Some(_token) = ui
            .modal_popup_config(&self.tag)
            .resizable(false)
            .movable(false)
            .title_bar(false)
            .scroll_bar(false)
            .begin_popup()
        {
            for widget in &mut self.children {
                widget.interact(ui);
            }

            for widget in &mut self.children {
                widget.render(ui);
            }

            if ui.button_with_size(&self.label_close, [button_width, BUTTON_HEIGHT])
                || (self.key_close.is_pressed(ui) && !ui.is_any_item_active())
            {
                ui.close_current_popup();
            }
        }
    }

    fn render_closed(&mut self, ui: &imgui::Ui) {
        for widget in &mut self.children {
            widget.render_closed(ui);
        }
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        for widget in &mut self.children {
            widget.interact(ui);
        }
    }

    fn log(&mut self, tx: Sender<String>) {
        for widget in &mut self.children {
            widget.log(tx.clone());
        }
    }
}
