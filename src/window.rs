//! The GUI window: a link box, an output folder box and a Download button.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use eframe::egui;

use crate::downloader;

/// Messages sent from the download thread back to the GUI.
enum Update {
    Progress(String),
    Done(Result<(), String>),
}

pub struct App {
    url: String,
    output_dir: String,
    status: String,
    downloading: bool,
    rx: Option<Receiver<Update>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            url: String::new(),
            output_dir: default_output_dir().display().to_string(),
            status: "Paste a YouTube link and press Download.".to_owned(),
            downloading: false,
            rx: None,
        }
    }
}

fn default_output_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    match home {
        Some(home) => PathBuf::from(home).join("Downloads"),
        None => PathBuf::from("."),
    }
}

impl App {
    fn start_download(&mut self, ctx: &egui::Context) {
        let (tx, rx) = mpsc::channel();
        let url = self.url.trim().to_owned();
        let output_dir = PathBuf::from(self.output_dir.trim());
        let ctx = ctx.clone();

        self.rx = Some(rx);
        self.downloading = true;
        self.status = "Starting download...".to_owned();

        thread::spawn(move || {
            let result = downloader::download_mp3(&url, &output_dir, |line| {
                let _ = tx.send(Update::Progress(line));
                ctx.request_repaint();
            });
            let _ = tx.send(Update::Done(result));
            ctx.request_repaint();
        });
    }

    fn poll_download(&mut self) {
        let Some(rx) = &self.rx else { return };
        while let Ok(update) = rx.try_recv() {
            match update {
                Update::Progress(line) => self.status = line,
                Update::Done(Ok(())) => {
                    self.status = format!("Done! Saved to {}", self.output_dir);
                    self.downloading = false;
                }
                Update::Done(Err(err)) => {
                    self.status = format!("Error: {err}");
                    self.downloading = false;
                }
            }
        }
        if !self.downloading {
            self.rx = None;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_download();
        let ctx = ui.ctx().clone();

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("YouTube to MP3");
            ui.add_space(8.0);

            ui.label("Video link:");
            let url_box = ui.add_enabled(
                !self.downloading,
                egui::TextEdit::singleline(&mut self.url)
                    .hint_text("https://www.youtube.com/watch?v=...")
                    .desired_width(f32::INFINITY),
            );

            ui.label("Save to folder:");
            ui.add_enabled(
                !self.downloading,
                egui::TextEdit::singleline(&mut self.output_dir).desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);

            let can_download = !self.downloading && !self.url.trim().is_empty();
            let enter_pressed =
                url_box.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                let clicked = ui
                    .add_enabled(can_download, egui::Button::new("Download"))
                    .clicked();
                if (clicked || enter_pressed) && can_download {
                    self.start_download(&ctx);
                }
                if self.downloading {
                    ui.spinner();
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label(&self.status);
        });
    }
}

/// Opens the window and runs until it is closed.
pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("YouTube to MP3")
            .with_inner_size([520.0, 260.0]),
        ..Default::default()
    };
    eframe::run_native(
        "YouTube to MP3",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
