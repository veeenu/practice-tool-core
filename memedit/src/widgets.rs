//! Flag widgets driven by [`FlagToggler`]s.

use practice_tool_core::key::Key;
use practice_tool_core::widgets::flag::{Flag, FlagWidget};
use practice_tool_core::widgets::Widget;

use crate::FlagToggler;

struct SingleFlag<F>(F);

impl<F: FlagToggler> Flag for SingleFlag<F> {
    fn set(&mut self, value: bool) {
        self.0.set(value);
    }

    fn get(&self) -> Option<bool> {
        self.0.get()
    }
}

struct MultiFlag<F>(Vec<F>);

impl<F: FlagToggler> Flag for MultiFlag<F> {
    fn set(&mut self, value: bool) {
        for flag in &self.0 {
            flag.set(value);
        }
    }

    fn get(&self) -> Option<bool> {
        self.0.first().and_then(|x| x.get())
    }
}

pub fn flag_widget<F: FlagToggler + 'static>(
    label: &str,
    flag: F,
    key: Option<Key>,
) -> Box<dyn Widget> {
    Box::new(FlagWidget::new(label, SingleFlag(flag), key))
}

/// A single widget driving all of `flags`. It reads as the first one.
pub fn multi_flag<F: FlagToggler + 'static>(
    label: &str,
    flags: Vec<F>,
    key: Option<Key>,
) -> Box<dyn Widget> {
    Box::new(FlagWidget::new(label, MultiFlag(flags), key))
}
