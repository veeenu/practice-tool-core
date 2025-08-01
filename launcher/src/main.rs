use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Child;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use eframe::{App, NativeOptions};
use egui::{CentralPanel, ViewportBuilder};
use rfd::FileDialog;
use steamworks::{AppId, Client};

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

struct Launcher {
    app_path: String,
    tool_path: PathBuf,
    compat: Compat,
}

impl Launcher {
    fn for_appid(
        app_id: u32,
        tool_path: impl AsRef<Path>,
        app_path: Option<String>,
        app_subpath: &str,
    ) -> Result<Self> {
        let app_id = AppId(app_id);
        let client = Client::init_app(app_id)?;
        let compat = Compat::new(&client)?;
        let tool_path = tool_path.as_ref().to_path_buf();

        let app_path = app_path.unwrap_or_else(|| {
            PathBuf::from(client.apps().app_install_dir(app_id))
                .join(app_subpath)
                .to_string_lossy()
                .to_string()
        });

        Ok(Self { app_path, tool_path, compat })
    }

    fn launch(&self) -> Result<()> {
        println!("Launching {:?}", self.app_path);
        let _ = self.compat.launch(&self.app_path, "run");
        thread::sleep(Duration::from_millis(5000));
        self.launch_tool()
    }

    fn launch_tool(&self) -> Result<()> {
        let _ = self.compat.launch(&self.tool_path, "runinprefix");

        Ok(())
    }
}

impl App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Path to game:");
                ui.add(egui::TextEdit::singleline(&mut self.app_path).interactive(false));
                if ui.button("Browse...").clicked() {
                    if let Some(path) = FileDialog::new().pick_file() {
                        self.app_path = path.to_string_lossy().to_string();
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Launch").clicked() {
                    self.launch();
                }

                if ui.button("Launch tool").clicked() {
                    self.launch();
                }
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
                Launcher::for_appid(
                    1245620,
                    "./jdsd_er_practice_tool.exe",
                    None,
                    "Game/eldenring.exe",
                )
                .unwrap(),
            ))
        }),
    )
}
