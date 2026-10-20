# Youtube to MP3 in Rust
A program for downloading the audio of youtube videos in an mp3 format.

Paste a YouTube link into the window and the app shows the video's thumbnail,
title, channel, length, views, likes and upload date. Press **Download** to save
the audio as an MP3.

## Requirements

- **Rust** 1.85 or newer, to build the app. Install it from <https://rustup.rs>.
- **yt-dlp**, which downloads the video.
- **ffmpeg**, which yt-dlp uses to convert the audio to MP3.

yt-dlp and ffmpeg must be on your `PATH`.

### Installing yt-dlp and ffmpeg

**Windows** (winget installs ffmpeg along with yt-dlp):

```bash
winget install --id yt-dlp.yt-dlp -e
```

**macOS** (Homebrew):

```bash
brew install yt-dlp ffmpeg
```

**Linux** (Debian/Ubuntu; your distribution's yt-dlp package may be old, so
`pipx install yt-dlp` is a good alternative):

```bash
sudo apt install yt-dlp ffmpeg
```

Open a new terminal afterwards, then check that both are found:

```bash
yt-dlp --version
```

```bash
ffmpeg -version
```

## Getting the code

Clone the repository:

```bash
git clone https://github.com/AmritSanjeev2007/youtube-2-mp3-rust.git
```

```bash
cd youtube-2-mp3-rust
```

Or download it from GitHub with **Code → Download ZIP**, extract it, and open a
terminal in the extracted folder.

## Running the app

From the project folder:

```bash
cargo run --release
```

The first build downloads and compiles the dependencies, which takes a few
minutes. Later runs start straight away.

To get a standalone program instead, build it once:

```bash
cargo build --release
```

The program is then at `target/release/youtube_mp3.exe` on Windows (or
`target/release/youtube_mp3` on macOS and Linux). You can copy it anywhere and
start it by double-clicking; yt-dlp and ffmpeg still need to be on your `PATH`.

## Using the app

1. Paste a YouTube link into **Video link**. After a moment, the video's
   thumbnail and details appear.
2. Change **Save to folder** if you don't want the MP3 in your Downloads folder.
3. Press **Download** (or Enter in the link box). Progress is shown at the
   bottom of the window, and the MP3 is named after the video's title.

## Troubleshooting

- **"yt-dlp was not found"**: yt-dlp isn't on your `PATH`. Install it as above,
  then close and reopen your terminal (or log out and back in) so the new
  `PATH` is picked up.
- **Download fails during conversion**: ffmpeg is missing. Check with
  `ffmpeg -version`.
- **"Couldn't load video info" or downloads failing for videos that play in a
  browser**: YouTube changes often, so update yt-dlp. With winget, run
  `winget upgrade yt-dlp.yt-dlp`; otherwise run `yt-dlp -U`.
