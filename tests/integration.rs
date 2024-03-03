use imgui::Ui;
use practice_tool_core::widgets::{
    flag::{Flag, FlagWidget},
    Widget,
};

mod harness;

struct TestWidget<W>(W);

impl<W: Widget> harness::Test for TestWidget<W> {
    fn render(&mut self, ui: &Ui) -> bool {
        self.0.render(ui);
        self.0.interact(ui);

        true
    }
}

fn test_flag() {
    struct TestFlag(bool);

    impl Flag for TestFlag {
        fn set(&mut self, value: bool) {
            self.0 = value;
        }

        fn get(&self) -> Option<bool> {
            Some(self.0)
        }
    }

    // TODO
    // These all activate when pressing ctrl+lalt+rshift+f because they technically match.
    // Does it make sense to make this more restrictive?
    harness::test(vec![
        TestWidget(FlagWidget::new("test 1", TestFlag(true), "ctrl+f".parse().ok())),
        TestWidget(FlagWidget::new("test 2", TestFlag(true), "ctrl+alt+f".parse().ok())),
        TestWidget(FlagWidget::new("test 3", TestFlag(true), "ctrl+lalt+rshift+f".parse().ok())),
    ]);
}

fn main() {
    test_flag();
}
