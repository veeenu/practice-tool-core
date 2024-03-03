use imgui::Ui;

mod harness;

fn test() {
    struct Test {}
    impl harness::Test for Test {
        fn render(&mut self, ui: &Ui) -> bool {
            ui.show_user_guide();

            true
        }
    }

    harness::test(Test {})
}

fn main() {
    test();
}
