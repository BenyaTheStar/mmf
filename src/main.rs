#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use eframe::egui;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

const CONFIG_PATH: &str = "!mmf.yaml";

#[derive(Default, Deserialize, Serialize)]
struct Config {
    provider: String,
    #[serde(rename = "dark-theme")]
    dark_theme: bool,
    whitelist: Vec<String>,
}

impl Config {
    fn save(&self) -> Result<()> {
        fs::write(CONFIG_PATH, yaml_serde::to_string(&self)?)?;
        Ok(())
    }
}

#[derive(Default)]
struct ArcMutex<T>(Arc<Mutex<T>>);

impl<T: Clone> ArcMutex<T> {
    fn get(&self) -> Option<T> {
        match Arc::clone(&self.0).lock() {
            Ok(value) => Some(value.clone()),
            Err(_) => None,
        }
    }

    fn mutate<F>(&self, mut fun: F) -> Option<()>
    where
        F: FnMut(&mut T),
    {
        match Arc::clone(&self.0).lock() {
            Ok(mut value) => {
                fun(&mut value);
                Some(())
            }
            Err(_) => None,
        }
    }
}

#[derive(Default)]
struct Updater {
    update_in_progress: ArcMutex<bool>,
    to_be_downloaded: ArcMutex<HashMap<String, bool>>,
    to_be_deleted: ArcMutex<HashMap<String, bool>>,
}

#[derive(Default)]
struct App {
    config: Config,
    updater: Updater,
    rx: Option<mpsc::Receiver<()>>,
}

impl App {
    fn calulate_jars(&mut self) -> Result<()> {
        {
            let ref mut provider = self.config.provider;
            let mut save = false;

            if provider.ends_with("/") {
                provider.pop();
                save = true;
            }

            let mut prefix = "http://";
            if provider.starts_with(prefix) {
                provider.drain(0..prefix.len());
                save = true;
            }

            prefix = "https://";
            if provider.starts_with(prefix) {
                provider.drain(0..prefix.len());
                save = true;
            }

            if save {
                self.config.save().unwrap();
            }
        }

        let local_jars = fs::read_dir(".")?
            .par_bridge()
            .map(|res| res.map(|e| e.path().to_str().unwrap().to_string().split_off(2)))
            .filter(|x| x.as_ref().unwrap().ends_with(".jar"))
            .collect::<Result<HashSet<_>, _>>()?;

        let remote_jars =
            reqwest::blocking::get("https://".to_string() + &self.config.provider + "/mods.json")?
                .json::<HashSet<String>>()?;

        self.updater
            .to_be_downloaded
            .mutate(|tbd| {
                let to_be_downloaded = remote_jars.difference(&local_jars);

                *tbd = to_be_downloaded
                    .par_bridge()
                    .map(|key| (key.clone(), !self.config.whitelist.contains(key)))
                    .collect();
            })
            .unwrap();

        self.updater
            .to_be_deleted
            .mutate(|tbd| {
                let to_be_deleted = local_jars.difference(&remote_jars);

                *tbd = to_be_deleted
                    .par_bridge()
                    .map(|key| (key.clone(), !self.config.whitelist.contains(key)))
                    .collect();
            })
            .unwrap();

        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        self.updater
            .update_in_progress
            .mutate(|update_in_progress| *update_in_progress = true)
            .unwrap();

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        let to_be_downloaded = self.updater.to_be_downloaded.get().unwrap();
        let to_be_deleted = self.updater.to_be_deleted.get().unwrap();
        let url = "https://".to_string() + &self.config.provider + "/mods/";

        std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                futures::future::join_all(
                    to_be_downloaded
                        .par_iter()
                        .filter_map(|(mod_, needed)| {
                            let url = url.clone() + mod_;

                            if *needed {
                                Some(App::download(url, mod_))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                )
                .await;

                futures::future::join_all(
                    to_be_deleted
                        .par_iter()
                        .filter_map(|(mod_, needed)| {
                            if *needed {
                                Some(tokio::fs::remove_file(mod_))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                )
                .await;
            });

            tx.send(()).unwrap();
        });

        Ok(())
    }

    async fn download(url: String, path: &str) -> Option<()> {
        if let Ok(response) = reqwest::get(url).await {
            if response.status().is_success() {
                if let Ok(bytes) = response.bytes().await {
                    if let Ok(_) = fs::write(path, bytes) {
                        return Some(());
                    }
                }
            }
        }

        None
    }

    fn ui_update_in_progress(&mut self, ui: &mut egui::Ui) {
        ui.label("Módosítócsomag-frissítés...");
        ui.spinner();

        if self.rx.as_ref().unwrap().try_recv().is_ok() {
            self.updater
                .update_in_progress
                .mutate(|update_in_progress| *update_in_progress = false)
                .unwrap();

            self.calulate_jars().unwrap();
        }
    }

    fn ui_update_not_in_progress(&mut self, ui: &mut egui::Ui) {
        if ui
            .button(
                egui::RichText::new("Módosítócsomag-frissítés")
                    .size(15.0)
                    .line_height(Some(20.0)),
            )
            .clicked()
        {
            self.update().unwrap();
        }

        ui.add_space(10.0);

        ui.horizontal_centered(|ui| {
            self.ui_left_list(ui);
            self.ui_right_list(ui);

            let mut new_whitelist: Vec<String> = std::iter::chain(
                self.updater.to_be_downloaded.get().unwrap(),
                self.updater.to_be_deleted.get().unwrap(),
            )
            .par_bridge()
            .filter_map(|(key, tick)| if tick { None } else { Some(key) })
            .collect();

            new_whitelist.sort();

            if self.config.whitelist != new_whitelist {
                self.config.whitelist = new_whitelist;
                self.config.save().unwrap();
            }
        });
    }

    fn ui_left_list(&mut self, ui: &mut egui::Ui) {
        self.updater
            .to_be_downloaded
            .mutate(|tbd| {
                App::ui_list(
                    ui,
                    |ui| ui.set_width(ui.available_width() / 2.0 - 11.0),
                    "Letöltendő",
                    "Nincsenek letöltendő módosítócsomagok.",
                    tbd,
                    0,
                );
            })
            .unwrap();
    }

    fn ui_right_list(&mut self, ui: &mut egui::Ui) {
        self.updater
            .to_be_deleted
            .mutate(|tbd| {
                App::ui_list(
                    ui,
                    |_| {},
                    "Eltávolítandó",
                    "Nincsenek eltávolítandó módosítócsomagok.",
                    tbd,
                    1,
                );
            })
            .unwrap();
    }

    fn ui_list(
        ui: &mut egui::Ui,
        set_width: impl FnOnce(&mut egui::Ui),
        title: &str,
        text_when_empty: &str,
        tbd: &mut HashMap<String, bool>,
        id: impl egui::AsIdSalt,
    ) {
        ui.group(|ui| {
            ui.set_min_height(ui.available_height());

            ui.vertical(|ui| {
                set_width(ui);

                ui.vertical_centered(|ui| {
                    ui.label(title);
                });

                if tbd.len() > 0 {
                    ui.push_id(id, |ui| {
                        egui::ScrollArea::both().show(ui, |ui| {
                            ui.set_min_height(ui.available_height());
                            ui.set_width(ui.available_width());

                            for (mod_, mut tick) in {
                                let mut mods = tbd.iter_mut().collect::<Vec<_>>();
                                mods.sort_by_key(|(key, _)| key.to_uppercase());
                                mods
                            } {
                                ui.horizontal(|ui| {
                                    ui.checkbox(
                                        &mut tick,
                                        egui::RichText::new(mod_).line_height(Some(16.0)),
                                    );
                                });
                            }
                        });
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.label(text_when_empty);
                    });
                }
            });
        });
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(if self.config.dark_theme {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });

        let font_name = "MCMCSF-FONT";

        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            font_name.into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../assets/font.ttf"
            ))),
        );

        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, font_name.into());

        ctx.set_fonts(fonts);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Minecraft-módosítócsomag-frissítő");

                ui.label(
                    "A módosítócsomag-frissítővel pofonegyszerűen \
                naprakészen tarthatod Minecraft-módosítócsomagjaidat!",
                );

                if self.updater.update_in_progress.get().unwrap() {
                    self.ui_update_in_progress(ui);
                } else {
                    self.ui_update_not_in_progress(ui);
                }
            });
        });
    }
}

fn main() -> eframe::Result {
    let config = if let Ok(yaml_text) = fs::read_to_string(CONFIG_PATH) {
        yaml_serde::from_str(&yaml_text).unwrap_or(Config::default())
    } else {
        Config::default()
    };

    config.save().unwrap();

    let options = eframe::NativeOptions {
        centered: true,
        run_and_return: false,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 500.0])
            .with_min_inner_size([500.0, 400.0])
            .with_icon(unsafe {
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
                    .unwrap_unchecked()
            })
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "MMF",
        options,
        Box::new(|_| {
            Ok(Box::new({
                let mut app = App {
                    config,
                    ..Default::default()
                };
                app.calulate_jars().unwrap();
                app
            }))
        }),
    )
}
