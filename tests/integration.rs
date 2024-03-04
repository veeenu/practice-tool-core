use std::fmt::Write;
use std::fs;

use practice_tool_core::widgets::flag::{Flag, FlagWidget};
use practice_tool_core::widgets::group::Group;
use practice_tool_core::widgets::position::{Position, PositionStorage};
use practice_tool_core::widgets::savefile_manager::SavefileManager;
use practice_tool_core::widgets::stats_editor::{Datum, Stats, StatsEditor};
use practice_tool_core::widgets::store_value::{StoreValue, Write as PtWrite};
use practice_tool_core::widgets::Widget;

mod harness;

macro_rules! harness_test {
    ($($t:expr),+) => {
        harness::test(vec![
            $(Box::new({ $t })),*
        ])
    }
}

struct TestFlag(bool);

impl Flag for TestFlag {
    fn set(&mut self, value: bool) {
        self.0 = value;
    }

    fn get(&self) -> Option<bool> {
        Some(self.0)
    }
}

#[test]
fn test_flag() {
    let mut flag1 = FlagWidget::new("test 1", TestFlag(true), "ctrl+f".parse().ok());
    let mut flag2 = FlagWidget::new("test 2", TestFlag(true), "ctrl+shift+f".parse().ok());
    let mut flag3 = FlagWidget::new("test 3", TestFlag(true), "ctrl+lalt+rshift+f".parse().ok());

    harness_test! {
        move |ui| { flag1.render(ui); flag1.interact(ui); },
        move |ui| { flag2.render(ui); flag2.interact(ui); },
        move |ui| { flag3.render(ui); flag3.interact(ui); }
    };
}

#[test]
fn test_savefile_manager() {
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

    harness_test! {
        move |ui| {
            let file_path = savefile_manager.0.path().join("ER0000.sl2");
            savefile_manager.1.render(ui);
            ui.text(format!("File contains: {}", fs::read_to_string(file_path).unwrap()));
        }
    };
}

#[test]
fn test_group() {
    let flag1 = Box::new(FlagWidget::new("test 1", TestFlag(true), None));
    let flag2 = Box::new(FlagWidget::new("test 2", TestFlag(true), None));
    let flag3 = Box::new(FlagWidget::new("test 3", TestFlag(true), None));

    let mut group = Group::new("Test group", "escape".parse().unwrap(), vec![flag1, flag2, flag3]);

    harness_test! {
        move |ui| group.render(ui)
    };
}

#[test]
fn test_position() {
    static mut X: f64 = 0.0;

    struct DummyPositionStorage {
        stored: f64,
        label_current: String,
        label_stored: String,
    }

    impl PositionStorage for DummyPositionStorage {
        fn read(&mut self) {
            unsafe { X = self.stored };
        }

        fn write(&mut self) {
            self.stored = unsafe { X };
        }

        fn display_current(&mut self) -> &str {
            self.label_current.clear();
            write!(self.label_current, "{:.3}", unsafe { X }).ok();
            unsafe { X += 0.001 };

            &self.label_current
        }

        fn display_stored(&mut self) -> &str {
            self.label_stored.clear();
            write!(self.label_stored, "{:.3}", self.stored).ok();

            &self.label_stored
        }

        fn is_valid(&self) -> bool {
            (unsafe { X * 5. } as u32 % 2) == 1
        }
    }

    let mut position = Position::new(
        DummyPositionStorage {
            stored: 0.0,
            label_current: String::new(),
            label_stored: String::new(),
        },
        "h".parse().ok(),
        "rshift+h".parse().ok(),
    );

    harness_test! {
        move |ui| {
            position.render(ui);
            position.interact(ui);
        }
    }
}

#[test]
fn test_stats_editor() {
    static mut STATS: (i32, i32, f32) = (10, 10, 10.0);

    #[derive(Default)]
    struct CharacterStats {
        hp: i32,
        mp: i32,
        strength: f32,
        open: bool,
    }

    impl Stats for CharacterStats {
        fn data(&mut self) -> Option<impl Iterator<Item = Datum>> {
            if self.open {
                Some(
                    [
                        Datum::int("HP", &mut self.hp, 1, 99),
                        Datum::int("MP", &mut self.mp, 1, 99),
                        Datum::float("Strength", &mut self.strength, 1.0, 199.9),
                    ]
                    .into_iter(),
                )
            } else {
                None
            }
        }

        fn read(&mut self) {
            self.open = true;
            (self.hp, self.mp, self.strength) = unsafe { STATS };
        }

        fn write(&mut self) {
            unsafe { STATS = (self.hp, self.mp, self.strength) };
        }

        fn clear(&mut self) {
            self.open = false;
        }
    }

    let mut stats_editor = StatsEditor::new(CharacterStats::default(), "escape".parse().ok());

    harness_test! {
        move |ui| {
            stats_editor.render(ui);
            stats_editor.interact(ui);
        }
    }
}

#[test]
fn test_store_value() {
    static mut QUITOUTS: usize = 0;

    struct QuitoutWrite;
    impl PtWrite for QuitoutWrite {
        fn write(&mut self) {
            unsafe { QUITOUTS += 1 };
        }
    }

    let mut store_value = StoreValue::new(QuitoutWrite, "Quitout", "p".parse().ok());

    harness_test! {
        move |ui| {
            store_value.render(ui);
            store_value.interact(ui);
            ui.text(format!("Quit out {} times", unsafe { QUITOUTS }));
        }
    }
}
