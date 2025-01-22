use std::cmp::Ordering;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, io};

use crossbeam_channel::Sender;
use imgui::sys::{
    igGetCursorPosX, igGetCursorPosY, igGetTreeNodeToLabelSpacing, igGetWindowPos, igIndent,
    igSetNextWindowPos, igUnindent, ImVec2,
};
use imgui::{Condition, TreeNodeFlags, Ui};

use crate::key::Key;
use crate::widgets::{scaling_factor, Widget, BUTTON_HEIGHT, BUTTON_WIDTH};

const SFM_TAG: &str = "##savefile-manager";
const SFML_TAG: &str = "##savefile-manager-list";

pub struct SavefileManager(Box<dyn Widget>);

impl SavefileManager {
    pub fn new(key_load: Option<Key>, key_close: Option<Key>, savefile_path: PathBuf) -> Self {
        match SavefileManagerInner::new(key_load, key_close, savefile_path) {
            Ok(savefile_manager) => SavefileManager(Box::new(savefile_manager)),
            Err(e) => SavefileManager(Box::new(ErroredSavefileManager(e))),
        }
    }
}

impl Widget for SavefileManager {
    fn render(&mut self, ui: &imgui::Ui) {
        self.0.render(ui)
    }

    fn render_closed(&mut self, ui: &imgui::Ui) {
        self.0.render_closed(ui)
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        self.0.interact(ui)
    }

    fn action(&mut self) {
        self.0.action()
    }

    fn cursor_down(&mut self) {
        self.0.cursor_down()
    }

    fn cursor_up(&mut self) {
        self.0.cursor_up()
    }

    fn want_enter(&mut self) -> bool {
        self.0.want_enter()
    }

    fn want_exit(&mut self) -> bool {
        self.0.want_exit()
    }

    fn log(&mut self, tx: Sender<String>) {
        self.0.log(tx)
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
    label_load: String,
    label_close: String,
    key_load: Option<Key>,
    key_close: Option<Key>,
    file_tree: FileTree,
    savefile_path: PathBuf,
    current_file: Option<PathBuf>,
    breadcrumbs: String,
    savefile_name: String,
    input_edited: bool,
    logs: Vec<String>,
}

impl SavefileManagerInner {
    fn new(
        key_load: Option<Key>,
        key_close: Option<Key>,
        savefile_path: PathBuf,
    ) -> Result<Self, String> {
        let label_load = match key_load {
            Some(key_load) => format!("Load savefile ({key_load})"),
            None => "Load savefile".to_string(),
        };

        let label_close = match key_close {
            Some(key_close) => format!("Close ({key_close})"),
            None => "Close".to_string(),
        };

        let Some(savefile_path_parent) = savefile_path.parent() else {
            return Err(format!(
                "Couldn't construct file browser: {savefile_path:?} has no parent"
            ));
        };

        let file_tree = FileTree::new(savefile_path_parent.to_path_buf())
            .map_err(|e| format!("Couldn't construct file browser: {}", e))?;

        Ok(SavefileManagerInner {
            label_load,
            label_close,
            key_load,
            key_close,
            file_tree,
            current_file: None,
            savefile_path,
            savefile_name: String::new(),
            breadcrumbs: "/".to_string(),
            input_edited: false,
            logs: Vec::new(),
        })
    }

    fn load_savefile(&mut self) {
        let Some(src_path) = self.current_file.as_ref() else {
            self.logs.push("No current path! Can't load savefile.".to_string());
            return;
        };

        if !src_path.is_file() {
            self.logs.push("Can't load a directory -- please choose a file.".to_string());
            return;
        }

        match load_savefile(src_path, &self.savefile_path) {
            Ok(()) => self.logs.push(format!(
                "Loaded {}/{}",
                if self.breadcrumbs == "/" { "" } else { &self.breadcrumbs },
                src_path.file_name().unwrap().to_str().unwrap()
            )),
            Err(e) => self.logs.push(format!("Error loading savefile: {}", e)),
        };
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

        let mut dst_path =
            self.current_file
                .as_ref()
                .and_then(|c| {
                    if c.is_dir() {
                        Some(c.clone())
                    } else {
                        c.parent().map(Path::to_path_buf)
                    }
                })
                .unwrap_or_else(|| self.file_tree.path().to_path_buf());
        dst_path.push(&self.savefile_name);

        match import_savefile(&dst_path, &self.savefile_path) {
            Ok(()) => {
                self.savefile_name.clear();
                if let Err(e) = self.file_tree.refresh() {
                    self.logs.push(format!("Couldn't refresh file tree: {e}"));
                }
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
    fn render(&mut self, ui: &Ui) {
        let scale = scaling_factor(ui);
        let button_width = BUTTON_WIDTH * scale;

        let (x, y) = unsafe {
            let mut wnd_pos = ImVec2::default();
            igGetWindowPos(&mut wnd_pos);
            (igGetCursorPosX() + wnd_pos.x, igGetCursorPosY() + wnd_pos.y)
        };

        if ui.button_with_size(&self.label_load, [button_width, BUTTON_HEIGHT]) {
            ui.open_popup(SFM_TAG);
            if let Err(e) = self.file_tree.refresh() {
                self.logs.push(format!("Couldn't refresh file tree: {e}"));
            }
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

            ui.child_window(SFML_TAG).size([button_width, 200. * scale]).build(|| {
                if self.file_tree.render(ui, &mut self.current_file, true) {
                    let root_path = self.file_tree.path();
                    let child_path = self
                        .current_file
                        .as_ref()
                        .and_then(|f| if f.is_dir() { Some(f.as_path()) } else { f.parent() })
                        .and_then(|path| path.strip_prefix(root_path).ok());

                    self.breadcrumbs.clear();

                    if let Some(path) = child_path {
                        write!(self.breadcrumbs, "/{}", path.to_string_lossy()).ok();
                    } else {
                        write!(self.breadcrumbs, "/").ok();
                    }
                }
            });

            if ui.button_with_size(&self.label_load, [button_width, BUTTON_HEIGHT]) {
                self.load_savefile();
            }

            ui.separator();

            {
                let _tok = ui.push_item_width(button_width * 174. / 240.);
                ui.input_text("##savefile_name", &mut self.savefile_name).hint("file name").build();
                self.input_edited = ui.is_item_active();
            }

            ui.same_line();

            if ui.button_with_size("Import", [button_width * 58. / 240., BUTTON_HEIGHT]) {
                self.import_savefile();
            }

            ui.separator();

            if ui.button_with_size("Show folder", [button_width, BUTTON_HEIGHT]) {
                let path = self
                    .current_file
                    .as_ref()
                    .map(|p| p as &Path)
                    .unwrap_or_else(|| self.file_tree.path());

                let path = if path.is_dir() { path } else { path.parent().unwrap() };

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

            if ui.button_with_size(&self.label_close, [button_width, BUTTON_HEIGHT])
                || (!ui.is_any_item_active()
                    && self.key_close.map(|k| k.is_pressed(ui)).unwrap_or(false))
            {
                ui.close_current_popup();
                if let Err(e) = self.file_tree.refresh() {
                    self.logs.push(format!("Couldn't refresh file tree: {e}"));
                }
            }
        }
    }

    fn interact(&mut self, ui: &imgui::Ui) {
        if self.key_load.map(|k| k.is_pressed(ui)).unwrap_or(false) {
            self.load_savefile();
        }
    }

    fn action(&mut self) {
        self.load_savefile();
    }

    fn log(&mut self, tx: Sender<String>) {
        for log in self.logs.drain(..) {
            tx.send(log).ok();
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FileTree {
    File { path: PathBuf },
    Directory { path: PathBuf, children: Vec<FileTree> },
}

impl FileTree {
    fn new(path: PathBuf) -> Result<Self, io::Error> {
        match fs::metadata(&path)? {
            m if m.file_type().is_file() => Ok(FileTree::File { path }),
            m if m.file_type().is_dir() => {
                let mut children = path
                    .read_dir()?
                    .map(|dir| FileTree::new(dir?.path()))
                    .collect::<Result<Vec<FileTree>, _>>()?;

                children.sort();

                Ok(FileTree::Directory { path, children })
            },
            m => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Unrecognized file type for {path:?}: {m:?}"),
            )),
        }
    }

    fn refresh(&mut self) -> Result<(), io::Error> {
        if let FileTree::Directory { path, .. } = self {
            *self = FileTree::new(path.to_path_buf())?;
        }

        Ok(())
    }

    fn file_name(&self) -> &str {
        match self {
            FileTree::File { path } | FileTree::Directory { path, .. } => {
                path.file_name().unwrap().to_str().unwrap()
            },
        }
    }

    fn path(&self) -> &Path {
        match self {
            FileTree::File { path } | FileTree::Directory { path, .. } => path,
        }
    }

    fn render(&self, ui: &Ui, current_file: &mut Option<PathBuf>, is_top: bool) -> bool {
        match self {
            FileTree::File { path } => {
                let is_current = current_file.as_ref().map(|f| f == path).unwrap_or(false);
                let file_name = self.file_name();

                unsafe { igUnindent(igGetTreeNodeToLabelSpacing()) };

                ui.tree_node_config(file_name)
                    .label::<&str, &str>(file_name)
                    .flags(
                        TreeNodeFlags::LEAF
                            | TreeNodeFlags::NO_TREE_PUSH_ON_OPEN
                            | TreeNodeFlags::SPAN_AVAIL_WIDTH,
                    )
                    .selected(is_current)
                    .build(|| {});

                unsafe { igIndent(igGetTreeNodeToLabelSpacing()) };

                if ui.is_item_clicked() {
                    *current_file = Some(path.clone());
                    true
                } else {
                    false
                }
            },
            FileTree::Directory { children, path } => {
                let is_current = current_file.as_ref().map(|f| f == path).unwrap_or(false);
                let file_name = self.file_name();
                let mut update_breadcrumbs = false;

                let node = ui
                    .tree_node_config(file_name)
                    .default_open(is_top)
                    .label::<&str, &str>(file_name)
                    .flags(TreeNodeFlags::SPAN_AVAIL_WIDTH)
                    .selected(is_current)
                    .build(|| {
                        if ui.is_item_clicked() {
                            *current_file = Some(path.clone());
                        }

                        for node in children {
                            update_breadcrumbs |= node.render(ui, current_file, false);
                        }
                    });

                if node.is_none() && ui.is_item_clicked() {
                    *current_file = Some(path.clone());
                    true
                } else {
                    update_breadcrumbs
                }
            },
        }
    }
}

impl PartialOrd for FileTree {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileTree {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (FileTree::File { path: a }, FileTree::File { path: b }) => a.cmp(b),
            (FileTree::File { .. }, FileTree::Directory { .. }) => Ordering::Greater,
            (FileTree::Directory { .. }, FileTree::File { .. }) => Ordering::Less,
            (FileTree::Directory { path: a, .. }, FileTree::Directory { path: b, .. }) => a.cmp(b),
        }
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
