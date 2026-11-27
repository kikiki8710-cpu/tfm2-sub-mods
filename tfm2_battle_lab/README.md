# TFM2 Battle Lab

Standalone program that runs only the match (battle) part of Teamfight Manager 2, without starting the game: replays from your save, custom matches, a skill lab and rule edits, shown in your browser.
Workshop: <https://steamcommunity.com/sharedfiles/filedetails/?id=3846572460> · User guide: [`dist/README_EN.txt`](dist/README_EN.txt) (Korean: [`dist/README.txt`](dist/README.txt))

| Folder | What it is |
|---|---|
| `app/` | `battle_lab.exe`: local web server (127.0.0.1 only) and the browser UI (`app/ui`). It never loads game code. |
| `lab/` | The simulator. It loads the installed game's exe as a library and runs one match per process. Built as `battle_lab_host.exe`; the release ships it as `lab_worker.exe`. |
| `assets/` | Small name / layout tables embedded in the app. |

## Build

1. `cargo build --release` in `app/` and in `lab/` (no external crates).
2. Put `battle_lab.exe` and the lab exe renamed to `lab_worker.exe` in one folder and run `battle_lab.exe`.
   (The release app is built with `RUSTFLAGS="-C target-feature=+crt-static"` so it runs without the Visual C++ runtime.)

The simulator calls the game's own functions at fixed addresses for one game version (0.7.4 for this source). Other versions are refused by an exe size check.
No game files are included; graphics and battle code come from your installed game.
