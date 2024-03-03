use std::fs;

use practice_tool_core::widgets::flag::{Flag, FlagWidget};
use practice_tool_core::widgets::savefile_manager::SavefileManager;
use practice_tool_core::widgets::Widget;

mod harness;

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

    println!("{tmp_dir:?}");

    fs::write(tmp_dir.path().join("ER0000.sl2"), "ER0000.sl2").unwrap();
    fs::write(tmp_dir.path().join("foo.sl2"), "foo").unwrap();
    fs::write(tmp_dir.path().join("bar.sl2"), "bar").unwrap();
    fs::create_dir_all(tmp_dir.path().join("Any%")).unwrap();
    fs::write(tmp_dir.path().join("Any%").join("ER0001.sl2"), "ER0001.sl2").unwrap();
    fs::write(tmp_dir.path().join("Any%").join("foo1.sl2"), "foo1").unwrap();
    fs::write(tmp_dir.path().join("Any%").join("bar1.sl2"), "bar1").unwrap();
    fs::create_dir_all(tmp_dir.path().join("All Bosses")).unwrap();
    fs::write(tmp_dir.path().join("All Bosses").join("ER0002.sl2"), "ER0002.sl2").unwrap();
    fs::write(tmp_dir.path().join("All Bosses").join("foo2.sl2"), "foo2").unwrap();
    fs::write(tmp_dir.path().join("All Bosses").join("bar2.sl2"), "bar2").unwrap();
    fs::create_dir_all(tmp_dir.path().join("Glitchless")).unwrap();
    fs::write(tmp_dir.path().join("Glitchless").join("ER0003.sl2"), "ER0003.sl2").unwrap();
    fs::write(tmp_dir.path().join("Glitchless").join("foo3.sl2"), "foo3").unwrap();
    fs::write(tmp_dir.path().join("Glitchless").join("bar3.sl2"), "bar3").unwrap();

    let savefile_manager =
        SavefileManager::new(Some("ctrl+o".parse().unwrap()), tmp_dir.path().join("ER0000.sl2"));
    let mut savefile_manager = (tmp_dir, savefile_manager);

    tests.push(Box::new(move |ui| {
        let file_path = savefile_manager.0.path().join("ER0000.sl2");
        savefile_manager.1.render(ui);
        ui.text(format!("File contains: {}", fs::read_to_string(file_path).unwrap()));
    }));
}

fn main() {
    let mut tests = vec![];

    test_flag(&mut tests);
    test_savefile_manager(&mut tests);

    harness::test(tests);
}
