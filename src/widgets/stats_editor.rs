use imgui::sys::{igGetCursorPosX, igGetCursorPosY, igGetWindowPos, igSetNextWindowPos, ImVec2};
use imgui::{Condition, WindowFlags};

use crate::key::Key;
use crate::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

const STAT_EDIT_TAG: &str = "##stats_editor";

pub enum Datum<'a> {
    Int { label: &'a str, value: &'a mut i32, min: i32, max: i32 },
    Float { label: &'a str, value: &'a mut f32, min: f32, max: f32 },
    Byte { label: &'a str, value: &'a mut i8, min: i8, max: i8 },
    Separator,
}

impl<'a> Datum<'a> {
    pub fn int(label: &'a str, value: &'a mut i32, min: i32, max: i32) -> Self {
        Datum::Int { label, value, min, max }
    }

    pub fn float(label: &'a str, value: &'a mut f32, min: f32, max: f32) -> Self {
        Datum::Float { label, value, min, max }
    }

    pub fn byte(label: &'a str, value: &'a mut i8, min: i8, max: i8) -> Self {
        Datum::Byte { label, value, min, max }
    }

    pub fn separator() -> Self {
        Datum::Separator
    }
}

pub trait Stats: Send + Sync + 'static {
    fn data(&mut self) -> Option<impl Iterator<Item = Datum>>;
    fn read(&mut self);
    fn write(&mut self);
    fn clear(&mut self);
}

pub struct StatsEditor<S: Stats> {
    stats: S,
    key_open: Option<Key>,
    label_open: String,
    key_close: Option<Key>,
    label_close: String,
}

impl<S: Stats> StatsEditor<S> {
    pub fn new(stats: S, key_open: Option<Key>, key_close: Option<Key>) -> Self {
        let label_open = match key_open {
            Some(key_open) => format!("Edit stats ({key_open})"),
            None => "Edit stats".to_string(),
        };

        let label_close = match key_close {
            Some(key_close) => format!("Close ({key_close})"),
            None => "Close".to_string(),
        };

        Self { stats, key_close, label_close, key_open, label_open }
    }
}

impl<S: Stats> Widget for StatsEditor<S> {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;
        let button_height = BUTTON_HEIGHT;

        let (x, y) = unsafe {
            let mut wnd_pos = ImVec2::default();
            igGetWindowPos(&mut wnd_pos);
            (igGetCursorPosX() + wnd_pos.x, igGetCursorPosY() + wnd_pos.y)
        };

        if ui.button_with_size(&self.label_open, [button_width, button_height])
            || (self.key_open.map(|k| k.is_pressed(ui)).unwrap_or(false)
                && !ui.is_any_item_active())
        {
            self.stats.read();
        }

        let Some(data) = self.stats.data() else {
            return;
        };

        ui.open_popup(STAT_EDIT_TAG);

        unsafe {
            igSetNextWindowPos(
                ImVec2::new(x + 200. * scale, y),
                Condition::Always as i8 as _,
                ImVec2::new(0., 0.),
            )
        };

        if let Some(_token) = ui
            .modal_popup_config(STAT_EDIT_TAG)
            .flags(
                WindowFlags::NO_TITLE_BAR
                    | WindowFlags::NO_RESIZE
                    | WindowFlags::NO_MOVE
                    | WindowFlags::NO_SCROLLBAR,
            )
            .begin_popup()
        {
            let _tok = ui.push_item_width(150.);

            for datum in data {
                match datum {
                    Datum::Int { label, value, min, max } => {
                        if ui.input_int(label, value).build() {
                            *value = (*value).clamp(min, max);
                        }
                    },
                    Datum::Float { label, value, min, max } => {
                        if ui.input_float(label, value).build() {
                            *value = (*value).clamp(min, max);
                        }
                    },
                    Datum::Byte { label, value, min, max } => {
                        if ui.input_scalar(label, value).step(1).build() {
                            *value = (*value).clamp(min, max);
                        }
                    },
                    Datum::Separator => ui.separator(),
                }
            }

            if ui.button_with_size("Apply", [button_width, button_height]) {
                self.stats.write();
            }

            if ui.button_with_size(&self.label_close, [button_width, button_height])
                || (self.key_close.map(|k| k.is_pressed(ui)).unwrap_or(false)
                    && !ui.is_any_item_active())
            {
                ui.close_current_popup();
                self.stats.clear();
            }
        }
    }
}
