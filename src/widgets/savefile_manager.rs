use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::Sender,
};

use imgui::{
    sys::{igGetCursorPosX, igGetCursorPosY, igGetWindowPos, igSetNextWindowPos, ImVec2},
    Condition, ListBox,
};

use crate::key::Key;
use crate::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

const SFM_TAG: &str = "##savefile-manager";
const SFML_TAG: &str = "##savefile-manager-list";

pub struct SavefileManager(Box<dyn Widget>);

impl SavefileManager {
    pub fn new(key_load: Option<Key>, savefile_path: PathBuf) -> Self {
        match SavefileManagerInner::new(key_load, savefile_path) {
            Ok(savefile_manager) => SavefileManager(Box::new(savefile_manager)),
            Err(e) => SavefileManager(Box::new(ErroredSavefileManager(e))),
        }
    }
}

impl Widget for SavefileManager {
    fn render(&mut self, ui: &imgui::Ui) {
        self.0.render(ui)
    }
}

#[derive(Debug)]
struct ErroredSavefileManager(String);

impl Widget for ErroredSavefileManager {
    fn render(&mut self, ui: &imgui::Ui) {
        ui.text(&self.0);
    }
}

#[derive(Debug)]
struct SavefileManagerInner {
    label: String,
    key_load: Option<Key>,
    key_down: Key,
    key_up: Key,
    key_enter: Key,
    dir_stack: DirStack,
    savefile_path: PathBuf,
    breadcrumbs: String,
    savefile_name: String,
    input_edited: bool,
    logs: Vec<String>,
}

impl SavefileManagerInner {
    fn new(key_load: Option<Key>, savefile_path: PathBuf) -> Result<Self, String> {
        let label = match key_load {
            Some(key_load) => format!("Load savefile ({key_load})"),
            None => "Load savefile".to_string(),
        };

        let Some(savefile_path_parent) = savefile_path.parent() else {
            return Err(format!(
                "Couldn't construct file browser: {savefile_path:?} has no parent"
            ));
        };

        let dir_stack = DirStack::new(savefile_path_parent)
            .map_err(|e| format!("Couldn't construct file browser: {}", e))?;

        Ok(SavefileManagerInner {
            label,
            key_load,
            key_down: "down".parse().unwrap(),
            key_up: "up".parse().unwrap(),
            key_enter: "enter".parse().unwrap(),
            dir_stack,
            savefile_path,
            savefile_name: String::new(),
            breadcrumbs: "/".to_string(),
            input_edited: false,
            logs: Vec::new(),
        })
    }

    fn load_savefile(&mut self) {
        if let Some(src_path) = self.dir_stack.current() {
            if src_path.is_file() {
                match load_savefile(src_path, &self.savefile_path) {
                    Ok(()) => self.logs.push(format!(
                        "Loaded {}/{}",
                        if self.breadcrumbs == "/" { "" } else { &self.breadcrumbs },
                        src_path.file_name().unwrap().to_str().unwrap()
                    )),
                    Err(e) => self.logs.push(format!("Error loading savefile: {}", e)),
                };
            }
        } else {
            self.logs.push("No current path! Can't load savefile.".to_string());
        }
    }

    fn import_savefile(&mut self) {
        if self.savefile_name.is_empty() {
            self.logs.push(String::from("Cannot save to empty filename"));
            return;
        }
        if self.savefile_name.contains('/') || self.savefile_name.contains('\\') {
            self.logs.push(String::from("Savefile name cannot contain path separator"));
            return;
        }
        let mut dst_path = PathBuf::from(self.dir_stack.path());
        dst_path.push(&self.savefile_name);
        match import_savefile(&dst_path, &self.savefile_path) {
            Ok(()) => {
                self.savefile_name.clear();
                self.dir_stack.refresh();
                self.logs.push(format!(
                    "Imported {}/{}",
                    if self.breadcrumbs == "/" { "" } else { &self.breadcrumbs },
                    dst_path.file_name().unwrap().to_str().unwrap()
                ))
            },
            Err(e) => self.logs.push(format!("Error importing savefile: {}", e)),
        };
    }
}

impl Widget for SavefileManagerInner {
    fn render(&mut self, ui: &imgui::Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;
        let button_height = BUTTON_HEIGHT;

        let (x, y) = unsafe {
            let mut wnd_pos = ImVec2::default();
            igGetWindowPos(&mut wnd_pos);
            (igGetCursorPosX() + wnd_pos.x, igGetCursorPosY() + wnd_pos.y)
        };

        if ui.button_with_size("Savefile manager", [button_width, button_height]) {
            ui.open_popup(SFM_TAG);
            self.dir_stack.refresh();
        }

        unsafe {
            igSetNextWindowPos(
                ImVec2::new(x + 200. * scale, y),
                Condition::Always as i8 as _,
                ImVec2::new(0., 0.),
            )
        };

        if let Some(_token) = ui
            .modal_popup_config(SFM_TAG)
            .resizable(false)
            .movable(false)
            .title_bar(false)
            .scroll_bar(false)
            .begin_popup()
        {
            ui.child_window("##savefile-manager-breadcrumbs")
                .size([button_width, 20. * scale])
                .build(|| {
                    ui.text(&self.breadcrumbs);
                    ui.set_scroll_x(ui.scroll_max_x());
                });

            let center_scroll_y = if self.key_down.is_pressed(ui) {
                self.dir_stack.next();
                true
            } else if self.key_up.is_pressed(ui) {
                self.dir_stack.prev();
                true
            } else {
                false
            };

            if self.key_enter.is_pressed(ui) {
                self.dir_stack.enter();
            }

            ListBox::new(SFML_TAG).size([button_width, 200. * scale]).build(ui, || {
                if ui.selectable_config(".. Up one dir").build() {
                    self.dir_stack.exit();
                    self.breadcrumbs = self.dir_stack.breadcrumbs();
                    self.dir_stack.refresh();
                }

                let mut goto: Option<usize> = None;
                for (idx, is_selected, i) in self.dir_stack.values() {
                    if ui.selectable_config(i).selected(is_selected).build() {
                        goto = Some(idx);
                    }

                    if center_scroll_y && is_selected {
                        ui.set_scroll_here_y();
                    }
                }

                if let Some(idx) = goto {
                    self.dir_stack.goto(idx);
                    self.dir_stack.enter();
                    self.breadcrumbs = self.dir_stack.breadcrumbs();
                }
            });

            if ui.button_with_size(&self.label, [button_width, button_height]) {
                self.load_savefile();
            }

            ui.separator();

            {
                let _tok = ui.push_item_width(button_width * 174. / 240.);
                ui.input_text("##savefile_name", &mut self.savefile_name).hint("file name").build();
                self.input_edited = ui.is_item_active();
            }

            ui.same_line();

            if ui.button_with_size("Import", [button_width * 58. / 240., button_height]) {
                self.import_savefile();
            }

            ui.separator();

            if ui.button_with_size("Show folder", [button_width, button_height]) {
                let path = self.dir_stack.path().to_owned();
                let path = if path.is_dir() { &path } else { path.parent().unwrap() };

                if let Err(e) = Command::new("explorer.exe")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .arg(path.as_os_str())
                    .spawn()
                {
                    self.logs.push(format!("Couldn't show folder: {}", e));
                };
            }

            if ui.button_with_size("Close", [button_width, button_height]) {
                ui.close_current_popup();
                self.dir_stack.refresh();
            }
        }
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        if self.input_edited {
            return;
        }

        if !ui.is_any_item_active() && self.key_load.map(|k| k.is_pressed(ui)).unwrap_or(false) {
            self.load_savefile();
        }
    }

    fn log(&mut self, tx: Sender<String>) {
        for log in self.logs.drain(..) {
            tx.send(log).ok();
        }
    }
}

#[derive(Debug)]
struct DirEntry {
    list: Vec<(PathBuf, String)>,
    cursor: usize,
    path: PathBuf,
}

impl DirEntry {
    fn new(path: &Path, cursor: Option<usize>) -> DirEntry {
        let mut list = DirStack::ls(path).unwrap();

        list.sort_by(|a, b| {
            let (ad, bd) = (a.is_dir(), b.is_dir());

            if ad == bd {
                a.cmp(b)
            } else if ad && !bd {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });

        let list: Vec<_> = list
            .into_iter()
            .map(|a| {
                let repr = if a.is_dir() {
                    format!("+  {}", a.file_name().unwrap().to_str().unwrap())
                } else {
                    format!("   {}", a.file_name().unwrap().to_str().unwrap())
                };
                (a, repr)
            })
            .collect();

        let max_len = list.len();

        DirEntry { list, cursor: cursor.unwrap_or(0).min(max_len), path: PathBuf::from(path) }
    }

    fn values(&self, directories_only: bool) -> impl IntoIterator<Item = (usize, bool, &str)> {
        self.list
            .iter()
            .filter(move |(d, _)| !directories_only || d.is_dir())
            .enumerate()
            .map(|(i, f)| (i, i == self.cursor, f.1.as_str()))
    }

    fn current(&self) -> Option<&PathBuf> {
        self.list.get(self.cursor).as_ref().map(|i| &i.0)
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn goto(&mut self, idx: usize) {
        if idx < self.list.len() {
            self.cursor = idx;
        }
    }

    fn prev(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn next(&mut self) {
        self.cursor = usize::min(self.cursor + 1, self.list.len() - 1);
    }
}

#[derive(Debug)]
struct DirStack {
    top: DirEntry,
    stack: Vec<DirEntry>,
}

impl DirStack {
    fn new(path: &Path) -> Result<Self, String> {
        Ok(DirStack { top: DirEntry::new(path, None), stack: Vec::new() })
    }

    fn enter(&mut self) {
        let new_entry = self
            .stack
            .last()
            .unwrap_or(&self.top)
            .current()
            .filter(|current_entry| current_entry.is_dir())
            .map(|current_entry| DirEntry::new(current_entry, None));

        if let Some(e) = new_entry {
            self.stack.push(e);
        }
    }

    fn exit(&mut self) -> bool {
        if self.stack.is_empty() {
            true
        } else {
            self.stack.pop().unwrap();
            false
        }
    }

    fn breadcrumbs(&self) -> String {
        if self.stack.is_empty() {
            String::from("/")
        } else {
            let mut breadcrumbs = String::new();
            for e in &self.stack {
                breadcrumbs.push('/');
                breadcrumbs.push_str(e.path().file_name().unwrap().to_str().unwrap());
            }
            breadcrumbs
        }
    }

    fn values(&self) -> impl IntoIterator<Item = (usize, bool, &str)> {
        match self.stack.last() {
            Some(d) => d.values(false).into_iter(),
            None => self.top.values(true).into_iter(),
        }
    }

    fn current(&self) -> Option<&PathBuf> {
        self.stack.last().unwrap_or(&self.top).current()
    }

    fn path(&self) -> &PathBuf {
        self.stack.last().unwrap_or(&self.top).path()
    }

    fn goto(&mut self, idx: usize) {
        self.stack.last_mut().unwrap_or(&mut self.top).goto(idx);
    }

    fn prev(&mut self) {
        self.stack.last_mut().unwrap_or(&mut self.top).prev();
    }

    fn next(&mut self) {
        self.stack.last_mut().unwrap_or(&mut self.top).next();
    }

    fn refresh(&mut self) {
        if let Some(l) = self.stack.last_mut() {
            *l = DirEntry::new(l.path(), Some(l.cursor));
        } else {
            self.top = DirEntry::new(self.top.path(), Some(self.top.cursor));
        }
    }

    // TODO SAFETY
    // FS errors would be permission denied (which shouldn't happen but should be
    // reported) and not a directory (which doesn't happen because we checked
    // for is_dir). For the moment, I just unwrap.
    fn ls(path: &Path) -> Result<Vec<PathBuf>, String> {
        Ok(std::fs::read_dir(path)
            .map_err(|e| format!("{}", e))?
            .filter_map(Result::ok)
            .map(|f| f.path())
            .collect())
    }
}

fn load_savefile(src: &Path, dest: &Path) -> Result<(), std::io::Error> {
    let buf = std::fs::read(src)?;
    std::fs::write(dest, buf)?;
    Ok(())
}

fn import_savefile(src: &Path, dest: &Path) -> Result<(), std::io::Error> {
    let buf = std::fs::read(dest)?;
    std::fs::write(src, buf)?;
    Ok(())
}
