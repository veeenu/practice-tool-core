use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use eframe::{App, Frame, NativeOptions};
use egui::{
    Align, Button, CentralPanel, ComboBox, Id, Label, Layout, Modal, Sides, ViewportBuilder,
};
use rfd::FileDialog;
use steamlocate::{App as SteamApp, SteamDir};
use toml_edit::{Array, DocumentMut, Item, Table, Value, value};

const WINDOW_WIDTH: f32 = 480.0;
const WINDOW_HEIGHT: f32 = 240.0;

#[cfg(unix)]
struct Compat {
    proton_path: PathBuf,
    compat_data_path: PathBuf,
}

#[cfg(unix)]
impl Compat {
    fn new(steam_dir: &SteamDir, app: &SteamApp) -> Result<Self> {
        use std::env;

        use keyvalues_parser::Vdf;

        let mapping = steam_dir.compat_tool_mapping()?;

        let tool = if let Some(tool) = mapping.get(&app.app_id) {
            tool
        } else if let Some(tool) = mapping.get(&0) {
            tool
        } else {
            mapping
                .iter()
                .fold((0, None), |(top_prio, top_tool), (_, tool)| {
                    if let Some(prio) = tool.priority
                        && prio > top_prio
                    {
                        (prio, Some(tool))
                    } else {
                        (top_prio, top_tool)
                    }
                })
                .1
                .ok_or_else(|| anyhow!("Couldn't find a mapped compatibility tool"))?
        };

        let tool_name = tool.name.as_ref().ok_or_else(|| anyhow!("Compat tool has no name"))?;

        let prefixes = [
            "/usr/share/steam/compatibilitytools.d".to_string(),
            "/usr/local/share/steam/compatibilitytools.d".to_string(),
            format!("{}/.steam/root/compatibilitytools.d", env::var("HOME").unwrap()),
            format!("{}/.local/share/Steam/compatibilitytools.d", env::var("HOME").unwrap()),
        ];

        let proton_path = prefixes
            .iter()
            .filter_map(|prefix| {
                let rd = fs::read_dir(prefix);
                rd.ok()
            })
            .flatten()
            .filter_map(|d| {
                println!("{d:?}");
                d.ok()
            })
            .find_map(|dir| {
                let dir = dir.path();

                println!("Checking {:?}", dir);

                if !dir.is_dir() {
                    return None;
                }

                let vdf_path = dir.join("compatibilitytool.vdf");
                if !vdf_path.exists() {
                    return None;
                }

                let vdf = fs::read_to_string(vdf_path).ok()?;
                println!("{vdf}");

                let vdf = Vdf::parse(&vdf).ok()?;

                if vdf.key != "compatibilitytools" {
                    return None;
                }

                let compatibilitytools = vdf.value.get_obj()?;
                if compatibilitytools
                    .iter()
                    .find_map(
                        |(key, values)| if key == "compat_tools" { Some(values) } else { None },
                    )?
                    .iter()
                    .filter_map(|v| v.get_obj())
                    .find(|o| o.contains_key(tool_name.as_str()))
                    .is_some()
                {
                    println!("Found {tool_name} at {dir:?}");
                    Some(dir)
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow!("Couldn't find a proton named {tool_name}"))?
            .join("proton");

        let compat_data_path = steam_dir
            .library_paths()?
            .into_iter()
            .map(|path| path.join("steamapps").join("compatdata").join(app.app_id.to_string()))
            .find(|path| path.exists())
            .ok_or_else(|| anyhow!("Couldn't find a compat data path"))?;

        Ok(Self { proton_path, compat_data_path })
    }

    pub fn launch(&self, app_path: impl AsRef<Path>, run_mode: &str) -> Command {
        let app_path = app_path.as_ref();

        let mut cmd = Command::new(&self.proton_path);

        cmd.env("STEAM_COMPAT_DATA_PATH", &self.compat_data_path)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &self.proton_path.parent().unwrap())
            .arg(run_mode)
            .arg(app_path);

        if let Some(parent) = app_path.parent()
            && !parent.to_str().unwrap().is_empty()
        {
            println!("{parent:?}");
            cmd.current_dir(parent);
        }

        cmd
    }
}

// Windows compat is a no-op.
#[cfg(windows)]
pub struct Compat;

#[cfg(windows)]
impl Compat {
    pub fn new(_: &SteamDir, _: &SteamApp) -> Result<Self> {
        Ok(Self)
    }

    pub fn launch(&self, app_path: impl AsRef<Path>, _: &str) -> Command {
        let app_path = app_path.as_ref();

        let mut cmd = Command::new(app_path);

        cmd.current_dir(app_path.parent().unwrap());

        cmd
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
        let steam_dir = SteamDir::locate()?;
        let (game, game_lib) = steam_dir
            .find_app(launcher_config.game_appid)?
            .ok_or_else(|| anyhow!("Couldn't find Steam"))?;

        let compat = Compat::new(&steam_dir, &game)?;
        let tool_path = PathBuf::from(launcher_config.tool_exe_path);

        let game_path = game_lib
            .resolve_app_dir(&game)
            .join(launcher_config.game_exe_subpath)
            .to_string_lossy()
            .to_string();

        let mut chosen_game_path = 0;
        let mut game_paths = vec![game_path.clone()];

        let toml = fs::read_to_string(launcher_config.config_file_name)
            .context("Couldn't load configuration file")?;
        let toml = toml.parse::<DocumentMut>()?;

        if let Some(launcher) = toml.get("launcher") {
            if let Some(paths) = launcher.get("paths")
                && let Some(paths) = paths.as_array()
            {
                game_paths.extend(
                    paths
                        .into_iter()
                        .filter_map(|el| el.as_str())
                        .filter(|&el| el != game_path)
                        .map(|s| s.to_owned()),
                );
            }

            if let Some(chosen) = launcher.get("last_run")
                && let Some(chosen) = chosen.as_integer()
                && chosen > 0
                && chosen < game_paths.len() as i64
            {
                chosen_game_path = chosen as usize
            };
        }

        Ok(Self {
            game_paths,
            tool_path,
            compat,
            chosen_game_path,
            confirm_msg: None,
            error_msg: None,
            launcher_config,
        })
    }

    fn launch(&self) -> Result<()> {
        let game_path = &self.game_paths[self.chosen_game_path];
        println!("Launching {:?}", game_path);
        let _ = self.compat.launch(game_path, "run").spawn()?;

        Ok(())
    }

    fn launch_tool(&self) -> Result<()> {
        println!("Launching tool {:?}", self.tool_path);
        let _ = self.compat.launch(&self.tool_path, "runinprefix").arg("--inject").spawn()?;

        Ok(())
    }

    fn confirm_modal(&mut self, ctx: &egui::Context) {
        let resp = if let Some(&(msg, action)) = self.confirm_msg.as_ref() {
            Modal::new(Id::new("confirm_modal")).show(ctx, |ui| {
                ui.label(msg);
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

    fn error_modal(&mut self, ctx: &egui::Context) {
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

    fn save_config(&mut self) -> Result<()> {
        let toml = fs::read_to_string(self.launcher_config.config_file_name).map_err(|e| {
            anyhow!(
                "Couldn't load configuration file {}:\n{e}",
                self.launcher_config.config_file_name
            )
        })?;

        let mut toml = toml.parse::<DocumentMut>().map_err(|e| {
            anyhow!(
                "Couldn't parse configuration file {}:\n{e}",
                self.launcher_config.config_file_name
            )
        })?;

        if !toml.contains_key("launcher") {
            toml.insert("launcher", Item::Table(Table::new()));
        }

        let mut game_paths = Array::new();
        for p in &self.game_paths {
            game_paths.push(p);
        }

        toml["launcher"]["paths"] = Item::Value(Value::Array(game_paths));
        toml["launcher"]["last_run"] = value(self.chosen_game_path as i64);

        fs::write(self.launcher_config.config_file_name, toml.to_string()).map_err(|e| {
            anyhow!(
                "Couldn't save configuration file {}: {e}",
                self.launcher_config.config_file_name
            )
        })?;

        Ok(())
    }

    fn add_game_version(&mut self) {
        if let Some(path) = FileDialog::new().pick_file() {
            if path.ends_with(self.launcher_config.game_exe_subpath) {
                let path = path.to_string_lossy().to_string();
                self.game_paths.push(path);

                if let Err(e) = self.save_config() {
                    self.error_msg = Some(e.to_string())
                }
            } else {
                self.error_msg =
                    Some(format!("This is not a path to the game's executable:\n{path:?}"));
            }
        }
    }

    fn delete_game_version(&mut self) {
        if self.game_paths.len() > 1 {
            self.game_paths.remove(self.chosen_game_path);
            self.chosen_game_path = usize::min(self.chosen_game_path, self.game_paths.len() - 1);

            if let Err(e) = self.save_config() {
                self.error_msg = Some(e.to_string())
            }
        } else {
            self.error_msg = Some("There must be at least one game version added.".to_string());
        }
    }
}

impl App for LauncherUi {
    fn update(&mut self, ctx: &egui::Context, _: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            // Image::new(egui::include_image!("../tests/fixtures/er.jpg"))
            //     .max_width(WINDOW_WIDTH)
            //     .maintain_aspect_ratio(true)
            //     .paint_at(ui, Rect::from_two_pos(Pos2 { x: 0.0, y: 0.0 }, Pos2 { x:
            // WINDOW_WIDTH, y: WINDOW_HEIGHT }));

            ui.add_space(40.0);

            self.confirm_modal(ctx);
            self.error_modal(ctx);

            ui.label("Choose a game version:");
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui.button("\u{1f5d1}").clicked() {
                    self.confirm_msg =
                        Some(("Delete the selected version?", Action::DeleteVersion));
                }

                if ui.button("+").clicked() {
                    self.add_game_version();
                }

                let resp = ComboBox::from_id_salt("Game version")
                    .width(ui.available_width())
                    .truncate()
                    .selected_text(&self.game_paths[self.chosen_game_path])
                    .show_index(ui, &mut self.chosen_game_path, self.game_paths.len(), |i| {
                        &self.game_paths[i]
                    });

                if resp.changed()
                    && let Err(e) = self.save_config()
                {
                    self.error_msg = Some(e.to_string())
                }
            });

            ui.add_space(40.0);

            ui.vertical_centered(|ui| {
                let aw = ui.available_width();

                if ui.add_sized([aw * 0.4, 0.0], Button::new("Launch game")).clicked() {
                    if let Err(e) = self.save_config() {
                        self.error_msg = Some(e.to_string())
                    }

                    if let Err(e) = self.launch() {
                        self.error_msg = Some(e.to_string());
                    }
                }

                if ui.add_sized([aw * 0.4, 0.0], Button::new("Launch tool only")).clicked() {
                    if let Err(e) = self.save_config() {
                        self.error_msg = Some(e.to_string())
                    }

                    if let Err(e) = self.launch_tool() {
                        self.error_msg = Some(e.to_string());
                    }
                }
            });
        });
    }
}

pub fn run(title: &'static str, launcher_config: LauncherConfig) -> eframe::Result {
    let options = NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size((WINDOW_WIDTH, WINDOW_HEIGHT)),
        ..Default::default()
    };

    eframe::run_native(
        title,
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(LauncherUi::new(launcher_config).unwrap()))
        }),
    )
}
