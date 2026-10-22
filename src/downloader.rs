//! Downloads YouTube audio as MP3 by driving the yt-dlp program.
//! yt-dlp uses ffmpeg for the MP3 conversion, so both must be on PATH.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

/// Details about a video, as reported by yt-dlp.
#[derive(Clone, Debug)]
pub struct VideoInfo {
    pub title: String,
    pub channel: Option<String>,
    /// Length in seconds.
    pub duration: Option<u64>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    /// Upload date as YYYY-MM-DD.
    pub upload_date: Option<String>,
    pub thumbnail: Option<String>,
}

/// Builds a yt-dlp command that doesn't flash a console window on Windows.
fn yt_dlp() -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new("yt-dlp");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn spawn_error(e: std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::NotFound {
        "yt-dlp was not found. Install it and make sure it is on your PATH.".to_owned()
    } else {
        format!("failed to start yt-dlp: {e}")
    }
}

/// Returns the last non-empty line of yt-dlp's error output.
fn last_error_line(stderr: &[u8]) -> Option<String> {
    String::from_utf8_lossy(stderr)
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(str::to_owned)
}

/// Fetches the title, thumbnail and stats of the video at `url`
/// without downloading it.
pub fn fetch_info(url: &str) -> Result<VideoInfo, String> {
    let output = yt_dlp()
        .args(["--dump-json", "--no-playlist", "--skip-download", "--", url])
        .stdin(Stdio::null())
        .output()
        .map_err(spawn_error)?;

    if !output.status.success() {
        return Err(last_error_line(&output.stderr)
            .unwrap_or_else(|| format!("yt-dlp exited with {}", output.status)));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("couldn't read yt-dlp's video info: {e}"))?;
    let text = |key: &str| json[key].as_str().filter(|s| !s.is_empty()).map(str::to_owned);
    let number = |key: &str| json[key].as_f64().map(|n| n as u64);

    // The default thumbnail is often WebP; YouTube's hqdefault.jpg is
    // available for every video and loads faster.
    let thumbnail = match (json["extractor_key"].as_str(), text("id")) {
        (Some("Youtube"), Some(id)) => Some(format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg")),
        _ => text("thumbnail"),
    };

    let upload_date = text("upload_date").map(|d| match (d.get(..4), d.get(4..6), d.get(6..8)) {
        (Some(y), Some(m), Some(day)) if d.len() == 8 => format!("{y}-{m}-{day}"),
        _ => d,
    });

    Ok(VideoInfo {
        title: text("title").unwrap_or_else(|| "(untitled)".to_owned()),
        channel: text("channel").or_else(|| text("uploader")),
        duration: number("duration"),
        view_count: number("view_count"),
        like_count: number("like_count"),
        upload_date,
        thumbnail,
    })
}

/// Downloads the audio of `url` as an MP3 into `output_dir`, calling
/// `on_progress` with each line of yt-dlp's progress output.
pub fn download_mp3(
    url: &str,
    output_dir: &Path,
    mut on_progress: impl FnMut(String),
) -> Result<(), String> {
    std::fs::create_dir_all(output_dir).map_err(|e| format!("can't create folder: {e}"))?;

    let mut cmd = yt_dlp();
    cmd.args(["-x", "--audio-format", "mp3", "--audio-quality", "0"])
        .args(["--no-playlist", "--newline"])
        // Tag the MP3 with the title and artist, and use the thumbnail,
        // cropped from 16:9 to a centred square, as its cover art.
        .args(["--embed-metadata", "--embed-thumbnail", "--convert-thumbnails", "jpg"])
        .args([
            "--ppa",
            r#"ThumbnailsConvertor+FFmpeg_o:-c:v mjpeg -vf "crop='min(iw,ih)':'min(iw,ih)'""#,
        ])
        .arg("-P")
        .arg(output_dir)
        .args(["-o", "%(title)s.%(ext)s", "--", url])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(spawn_error)?;

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
