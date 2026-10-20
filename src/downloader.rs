//! Downloads YouTube audio as MP3 by driving the yt-dlp program.
//! yt-dlp uses ffmpeg for the MP3 conversion, so both must be on PATH.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

/// Downloads the audio of `url` as an MP3 into `output_dir`, calling
/// `on_progress` with each line of yt-dlp's progress output.
pub fn download_mp3(
    url: &str,
    output_dir: &Path,
    mut on_progress: impl FnMut(String),
) -> Result<(), String> {
    std::fs::create_dir_all(output_dir).map_err(|e| format!("can't create folder: {e}"))?;

    let mut cmd = Command::new("yt-dlp");
    cmd.args(["-x", "--audio-format", "mp3", "--audio-quality", "0"])
        .args(["--no-playlist", "--newline"])
        .arg("-P")
        .arg(output_dir)
        .args(["-o", "%(title)s.%(ext)s", "--", url])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Don't flash a console window for yt-dlp on Windows.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "yt-dlp was not found. Install it and make sure it is on your PATH.".to_owned()
        } else {
            format!("failed to start yt-dlp: {e}")
        }
    })?;

    // Collect stderr on its own thread so a full pipe can't block yt-dlp.
    let stderr = child.stderr.take().unwrap();
    let stderr_thread = thread::spawn(move || {
        let mut last_error = String::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if !line.trim().is_empty() {
                last_error = line;
            }
        }
        last_error
    });

    let stdout = child.stdout.take().unwrap();
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let line = line.trim();
        if !line.is_empty() {
            on_progress(line.to_owned());
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    let last_error = stderr_thread.join().unwrap_or_default();
    if status.success() {
        Ok(())
    } else if last_error.is_empty() {
        Err(format!("yt-dlp exited with {status}"))
    } else {
        Err(last_error)
    }
}
