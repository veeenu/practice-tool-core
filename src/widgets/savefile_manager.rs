use std::{
    cmp::Ordering,
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering as SyncOrdering},
        mpsc::Sender,
    },
};

use imgui::{
    sys::{
        igGetCursorPosX, igGetCursorPosY, igGetTreeNodeToLabelSpacing, igGetWindowPos, igIndent,
        igSetNextWindowPos, igUnindent, ImVec2,
    },
    Condition, ListBox, TreeNodeFlags, Ui,
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
    file_tree: FileTree,
    savefile_path: PathBuf,
    current_file: Option<PathBuf>,
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

        let file_tree = FileTree::new(savefile_path_parent.to_path_buf())
            .map_err(|e| format!("Couldn't construct file browser: {}", e))?;

        Ok(SavefileManagerInner {
            label,
            key_load,
            key_down: "down".parse().unwrap(),
            key_up: "up".parse().unwrap(),
            key_enter: "enter".parse().unwrap(),
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
        if let Some(src_path) = self.current_file.as_ref() {
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

        let mut dst_path = self
            .current_file
            .as_ref()
            .and_then(|c| c.parent().map(Path::to_path_buf))
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

        if ui.button_with_size(&self.label, [button_width, BUTTON_HEIGHT]) {
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
                if self.file_tree.render(ui, &mut self.current_file) {
                    let root_path = self.file_tree.path();
                    let child_path = self.current_file.as_ref().unwrap().parent().unwrap();
                    self.breadcrumbs = format!(
                        "/{}",
                        child_path.strip_prefix(root_path).unwrap().to_string_lossy()
                    );
                }
            });

            if ui.button_with_size(&self.label, [button_width, BUTTON_HEIGHT]) {
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

            if ui.button_with_size("Close", [button_width, BUTTON_HEIGHT]) {
                ui.close_current_popup();
                if let Err(e) = self.file_tree.refresh() {
                    self.logs.push(format!("Couldn't refresh file tree: {e}"));
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FileTree {
    File { path: PathBuf },
    Directory { path: PathBuf, children: Vec<FileTree> },
}

impl FileTree {
    fn new(path: PathBuf) -> Result<Self, std::io::Error> {
        if path.is_file() {
            Ok(FileTree::File { path })
        } else if path.is_dir() {
            let mut children = path
                .read_dir()?
                .map(|dir| FileTree::new(dir?.path()))
                .collect::<Result<Vec<FileTree>, _>>()?;

            children.sort();

            Ok(FileTree::Directory { path, children })
        } else {
            unreachable!("Savefile path is neither file nor directory");
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

    fn render(&self, ui: &Ui, current_file: &mut Option<PathBuf>) -> bool {
        match self {
            FileTree::File { path } => {
                let is_current = current_file.as_ref().map(|f| f == path).unwrap_or(false);
                let file_name = self.file_name();

                unsafe { igUnindent(igGetTreeNodeToLabelSpacing()) };
                ui.tree_node_config(file_name)
                    .label::<&str, &str>(file_name)
                    .flags(if is_current {
                        TreeNodeFlags::LEAF
                            | TreeNodeFlags::SELECTED
                            | TreeNodeFlags::NO_TREE_PUSH_ON_OPEN
                    } else {
                        TreeNodeFlags::LEAF | TreeNodeFlags::NO_TREE_PUSH_ON_OPEN
                    })
                    .build(|| {});

                unsafe { igIndent(igGetTreeNodeToLabelSpacing()) };

                if ui.is_item_clicked() {
                    *current_file = Some(path.clone());
                    true
                } else {
                    false
                }
            },
            FileTree::Directory { children, .. } => {
                let file_name = self.file_name();
                let mut update_breadcrumbs = false;

                ui.tree_node_config(file_name)
                    .label::<&str, &str>(file_name)
                    .flags(TreeNodeFlags::SPAN_AVAIL_WIDTH)
                    .build(|| {
                        for node in children {
                            update_breadcrumbs |= node.render(ui, current_file);
                        }
                    });

                update_breadcrumbs
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
