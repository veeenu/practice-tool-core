use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Child;
use std::process::Command;
use std::time::Duration;
use std::{fs, thread};

use anyhow::{Context, Result, bail};
use eframe::{App, Frame, NativeOptions};
use egui::{
    Align, Button, CentralPanel, ComboBox, Grid, Id, Label, Layout, Modal, Pos2, Rect, Response,
    Sense, Sides, Vec2, ViewportBuilder,
};
use rfd::FileDialog;
use steamworks::{AppId, Client};
use toml_edit::{DocumentMut, value};

#[cfg(unix)]
struct Compat {
    proton_path: PathBuf,
}

#[cfg(unix)]
impl Compat {
    fn new(client: &Client) -> Result<Self> {
        use std::env;
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        // This gets filled automatically when instantiating a [`steamworks::Client`].
        let compat_data_path = PathBuf::from(
            env::var("STEAM_COMPAT_DATA_PATH").context("Compat data path unavailable")?,
        );

        let config_info = File::open(compat_data_path.join("config_info"))?;

        // Parse the paths in the config_info file, to find one that is "nearby" the
        // `proton` script.
        let Some(proton_path) = BufReader::new(config_info)
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| {
                use std::path::PathBuf;

                let path = PathBuf::from(line);
                if path.exists() && path.is_dir() { Some(path) } else { None }
            })
            .find_map(|path| {
                use std::os::unix::fs::PermissionsExt;

                let proton_path = path.join("../../proton").canonicalize().ok()?;
                let meta = proton_path.metadata().ok()?;

                if meta.permissions().mode() & 0o111 != 0 { Some(proton_path) } else { None }
            })
        else {
            bail!("Could not find proton path");
        };

        Ok(Self { proton_path })
    }

    pub fn launch(&self, app_path: impl AsRef<Path>, arg: &str) -> Result<Child> {
        let app_path = app_path.as_ref();

        let cmd = Command::new(&self.proton_path)
            .current_dir(app_path.parent().unwrap())
            .arg(arg)
            .arg(app_path)
            .spawn()?;

        Ok(cmd)
    }
}

// Windows compat is a no-op.
#[cfg(windows)]
pub struct Compat;

#[cfg(windows)]
impl Compat {
    pub fn new(_: &Client) -> Result<Self> {
        Ok(Self)
    }

    pub fn launch(&self, app_path: impl AsRef<Path>, _: &str) -> Result {
        Command::new(app_path).current_dir(app_path.parent()).spawn()
    }
}

pub struct LauncherConfig {
    /// The game's Steam appid (e.g. 1245620)
    pub game_appid: u32,
    /// The name of the game's executable file (e.g. `eldenring.exe`)
    pub game_exe_name: &'static str,
    /// The path to the executable from the game's installation directory
    /// (e.g. `Game/eldenring.exe`)
    pub game_exe_subpath: &'static str,
    /// The name of the tool's configuration file
    /// (e.g. `jdsd_er_practice_tool.toml`)
    pub config_file_name: &'static str,
    pub tool_exe_path: &'static str,
}

#[derive(Clone, Copy)]
enum Action {
    DoNothing,
    DeleteVersion,
}

struct LauncherUi {
    chosen_game_path: usize,
    game_paths: Vec<String>,
    tool_path: PathBuf,
    compat: Compat,
    launcher_config: LauncherConfig,

    confirm_msg: Option<(&'static str, Action)>,
    error_msg: Option<String>,
}

impl LauncherUi {
    fn new(launcher_config: LauncherConfig) -> Result<Self> {
        let app_id = AppId(launcher_config.game_appid);
        let client = Client::init_app(app_id)?;
        let compat = Compat::new(&client)?;
        let tool_path = PathBuf::from(launcher_config.tool_exe_path);

        let game_path = PathBuf::from(client.apps().app_install_dir(app_id))
            .join(launcher_config.game_exe_subpath)
            .to_string_lossy()
            .to_string();
        let mut game_paths = vec![game_path];

        let toml = fs::read_to_string(launcher_config.config_file_name)?;
        if let Some(paths) = toml.parse::<DocumentMut>()?["launcher"]["paths"].as_array() {
            game_paths.extend(paths.into_iter().filter_map(|el| el.as_str()).map(|s| s.to_owned()));
        }

        Ok(Self {
            game_paths,
            tool_path,
            compat,
            chosen_game_path: 0,
            confirm_msg: None,
            error_msg: None,
            launcher_config,
        })
    }

    fn launch(&self) -> Result<()> {
        let game_path = &self.game_paths[self.chosen_game_path];
        println!("Launching {:?}", game_path);
        let _ = self.compat.launch(game_path, "run");
        thread::sleep(Duration::from_millis(5000));
        self.launch_tool()
    }

    fn launch_tool(&self) -> Result<()> {
        let _ = self.compat.launch(&self.tool_path, "runinprefix");

        Ok(())
    }

    fn confirm_modal(&mut self, ctx: &egui::Context, ui: &egui::Ui) {
        let resp = if let Some(&(msg, action)) = self.confirm_msg.as_ref() {
            Modal::new(Id::new("confirm_modal")).show(ctx, |ui| {
                ui.label("Delete this game version?");
                Sides::new().show(
                    ui,
                    |_| {},
                    |ui| {
                        if ui.button("No").clicked() {
                            Some(Action::DoNothing)
                        } else if ui.button("Yes").clicked() {
                            Some(action)
                        } else {
                            None
                        }
                    },
                )
            })
        } else {
            return;
        };

        if let ((), Some(action)) = resp.inner {
            self.confirm_msg = None;
            match action {
                Action::DoNothing => {},
                Action::DeleteVersion => self.delete_game_version(),
            }
        }
    }

    fn error_modal(&mut self, ctx: &egui::Context, ui: &egui::Ui) {
        let resp = if let Some(e) = self.error_msg.as_ref() {
            Modal::new(Id::new("modal_error")).show(ctx, |ui| {
                ui.heading("Error");
                ui.add(Label::new(e).wrap());
                ui.button("Ok")
            })
        } else {
            return;
        };

        if resp.inner.clicked() {
            self.error_msg = None;
        }
    }

    fn add_game_version(&mut self) {
        if let Some(path) = FileDialog::new().pick_file() {
            if path.ends_with(self.launcher_config.game_exe_subpath) {
                let path = path.to_string_lossy().to_string();
                self.game_paths.push(path);
            } else {
                self.error_msg =
                    Some(format!("This is not a path to the game's executable:\n{path:?}"));
            }
        }
    }

    fn delete_game_version(&mut self) {
        println!("Deleting {}", self.chosen_game_path);
        if self.game_paths.len() > 1 {
            self.game_paths.remove(self.chosen_game_path);
            self.chosen_game_path = usize::min(self.chosen_game_path, self.game_paths.len() - 1);
        } else {
            self.error_msg = Some("There must be at least one game version added.".to_string());
        }
    }
}

impl App for LauncherUi {
    fn update(&mut self, ctx: &egui::Context, _: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            self.confirm_modal(ctx, ui);
            self.error_modal(ctx, ui);

            ui.label("Choose a game version:");
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui.button("\u{1f5d1}").clicked() {
                    self.confirm_msg =
                        Some(("Delete the selected version?", Action::DeleteVersion));
                }

                if ui.button("+").clicked() {
                    self.add_game_version();
                }

                ComboBox::from_id_salt("Game version")
                    .width(ui.available_width())
                    .truncate()
                    .selected_text(&self.game_paths[self.chosen_game_path])
                    .show_index(ui, &mut self.chosen_game_path, self.game_paths.len(), |i| {
                        &self.game_paths[i]
                    });
            });

            ui.vertical_centered(|ui| {
                let aw = ui.available_width();

                if ui.add_sized([aw * 0.4, 0.0], Button::new("Launch game + tool")).clicked() {
                    self.launch();
                }

                if ui.add_sized([aw * 0.4, 0.0], Button::new("Launch tool only")).clicked() {
                    self.launch_tool();
                }

                ui.end_row();
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size((480.0, 240.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Launcher",
        options,
        Box::new(|_| {
            Ok(Box::new(
                LauncherUi::new(LauncherConfig {
                    game_appid: 1245620,
                    game_exe_name: "eldenring.exe",
                    game_exe_subpath: "Game/eldenring.exe",
                    config_file_name: "tests/fixtures/jdsd_er_practice_tool.toml",
                    tool_exe_path: "jdsd_er_practice_tool.exe",
                })
                .unwrap(),
            ))
        }),
    )
}
