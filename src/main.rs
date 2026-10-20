// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod downloader;
mod window;

fn main() -> eframe::Result {
    window::run()
}
