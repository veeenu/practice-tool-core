use imgui::Ui;
use practice_tool_core::widgets::{
    flag::{Flag, FlagWidget},
    savefile_manager::{self, SavefileManager},
    Widget,
};

mod harness;

struct TestWidget<W>(W);

type TestVec = Vec<Box<harness::Test>>;

fn test_flag(tests: &mut TestVec) {
    struct TestFlag(bool);

    impl Flag for TestFlag {
        fn set(&mut self, value: bool) {
            self.0 = value;
        }

        fn get(&self) -> Option<bool> {
            Some(self.0)
        }
    }

    let mut flag1 = FlagWidget::new("test 1", TestFlag(true), "ctrl+f".parse().ok());
    let mut flag2 = FlagWidget::new("test 2", TestFlag(true), "ctrl+alt+f".parse().ok());
    let mut flag3 = FlagWidget::new("test 3", TestFlag(true), "ctrl+lalt+rshift+f".parse().ok());

    // TODO
    // These all activate when pressing ctrl+lalt+rshift+f because they technically match.
    // Does it make sense to make this more restrictive?
    tests.push(Box::new(move |ui| flag1.render(ui)));
    tests.push(Box::new(move |ui| flag2.render(ui)));
    tests.push(Box::new(move |ui| flag3.render(ui)));
}

fn test_savefile_manager(tests: &mut TestVec) {
    let tmp_dir = tempfile::tempdir().unwrap();

    std::fs::write(tmp_dir.path().join("ER0000.sl2"), "ER0000.sl2").unwrap();
    std::fs::write(tmp_dir.path().join("foo.sl2"), "foo").unwrap();
    std::fs::write(tmp_dir.path().join("bar.sl2"), "bar").unwrap();

    let savefile_manager =
        SavefileManager::new(Some("ctrl+o".parse().unwrap()), tmp_dir.path().join("ER0000.sl2"));
    let mut savefile_manager = (tmp_dir, savefile_manager);

    tests.push(Box::new(move |ui| {
        let _ = savefile_manager.0.path(); // keep the tmp directory alive
        savefile_manager.1.render(ui);
    }));
}

fn main() {
    let mut tests = vec![];

    test_flag(&mut tests);
    test_savefile_manager(&mut tests);

    harness::test(tests);
}
