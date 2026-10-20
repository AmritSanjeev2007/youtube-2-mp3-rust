//! The GUI window: a link box with a video preview, an output folder box
//! and a Download button.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::downloader::{self, VideoInfo};

/// How long to wait after the link stops changing before fetching its info.
const FETCH_DELAY: Duration = Duration::from_millis(600);

/// Messages sent from the download thread back to the GUI.
enum Update {
    Progress(String),
    Done(Result<(), String>),
}

/// The video info fetched for a link, tagged with that link.
type InfoResult = (String, Result<VideoInfo, String>);

pub struct App {
    url: String,
    output_dir: String,
    status: String,
    downloading: bool,
    rx: Option<Receiver<Update>>,

    /// The link the preview shows (or is being fetched for).
    info_url: String,
    info: Option<VideoInfo>,
    info_rx: Option<Receiver<InfoResult>>,
    /// When the link was last edited, while a fetch is waiting on it.
    last_edit: Option<Instant>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            url: String::new(),
            output_dir: default_output_dir().display().to_string(),
            status: "Paste a YouTube link and press Download.".to_owned(),
            downloading: false,
            rx: None,
            info_url: String::new(),
            info: None,
            info_rx: None,
            last_edit: None,
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

/// Formats seconds as m:ss, or h:mm:ss for an hour or more.
fn format_duration(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Formats a number with thousands separators, e.g. 1234567 -> 1,234,567.
fn format_count(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
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

    /// Called when the link text changes: drops the old preview and
    /// schedules a fetch for the new link.
    fn link_changed(&mut self) {
        let url = self.url.trim();
        if url == self.info_url {
            return;
        }
        self.info = None;
        self.info_rx = None;
        self.info_url.clear();
        self.last_edit = (!url.is_empty()).then(Instant::now);
    }

    /// Starts fetching the preview once the link has been still for a moment,
    /// and picks up the result when it arrives.
    fn poll_info(&mut self, ctx: &egui::Context) {
        if let Some(edited) = self.last_edit {
            let waited = edited.elapsed();
            if waited < FETCH_DELAY {
                ctx.request_repaint_after(FETCH_DELAY - waited);
            } else {
                self.last_edit = None;
                self.start_fetch(ctx);
            }
        }

        let Some(rx) = &self.info_rx else { return };
        if let Ok((url, result)) = rx.try_recv() {
            self.info_rx = None;
            if url != self.info_url {
                return;
            }
            match result {
                Ok(info) => self.info = Some(info),
                Err(err) => self.status = format!("Couldn't load video info: {err}"),
            }
        }
    }

    fn start_fetch(&mut self, ctx: &egui::Context) {
        let (tx, rx) = mpsc::channel();
        let url = self.url.trim().to_owned();
        let ctx = ctx.clone();

        self.info_url = url.clone();
        self.info_rx = Some(rx);

        thread::spawn(move || {
            let result = downloader::fetch_info(&url);
            let _ = tx.send((url, result));
            ctx.request_repaint();
        });
    }

    fn show_preview(&self, ui: &mut egui::Ui) {
        if self.info_rx.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading video info...");
            });
            return;
        }
        let Some(info) = &self.info else { return };

        ui.horizontal_top(|ui| {
            if let Some(thumbnail) = &info.thumbnail {
                ui.add(
                    egui::Image::new(thumbnail.as_str())
                        .max_width(240.0)
                        .corner_radius(4.0),
                );
            }
            ui.vertical(|ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(&info.title).strong().size(16.0)).wrap(),
                );
                if let Some(channel) = &info.channel {
                    ui.label(channel);
                }
                ui.add_space(4.0);
                egui::Grid::new("video_stats").num_columns(2).show(ui, |ui| {
                    let mut row = |name: &str, value: Option<String>| {
                        if let Some(value) = value {
                            ui.weak(name);
                            ui.label(value);
                            ui.end_row();
                        }
                    };
                    row("Length", info.duration.map(format_duration));
                    row("Views", info.view_count.map(format_count));
                    row("Likes", info.like_count.map(format_count));
                    row("Uploaded", info.upload_date.clone());
                });
            });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_download();
        self.poll_info(&ctx);

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
            if url_box.changed() {
                self.link_changed();
            }

            ui.add_space(4.0);
            self.show_preview(ui);
            ui.add_space(4.0);

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
            .with_inner_size([640.0, 440.0])
            .with_min_inner_size([480.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "YouTube to MP3",
        options,
        Box::new(|cc| {
            // Lets egui::Image load the thumbnail straight from its URL.
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::default()))
        }),
    )
}
