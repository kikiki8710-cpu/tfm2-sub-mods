# TFM2 VR

Play Teamfight Manager 2 in a PC VR headset (OpenXR, tested with Quest 3 + Air Link).
Workshop: <https://steamcommunity.com/sharedfiles/filedetails/?id=3846488476>

| Folder | What it is |
|---|---|
| `bridge/` | Game mod `tfm2_vr_bridge` (stable mod API). It only copies the game's draw commands for the viewer and changes nothing in the game. |
| `viewer/` | `tfm2_vr_viewer.exe`, the OpenXR / Direct3D 11 viewer (matches on a 3D table, other screens on a curved screen). `tfm2_vr_cube` is a small headset test. |
| `viewer.toml` | Viewer settings shipped next to the exe. |
| `package/` | Readme and launch scripts that go into the release package. |

## Build

- Bridge: copy `<game folder>\mod-sdk-stable\mod-api-stable` into the repository root (see the main README), then `cargo build --release` in `bridge/`.
- Viewer: `cargo build --release` in `viewer/` (Windows only).

The release package also contains `openxr_loader.dat`, the Khronos OpenXR loader (Apache 2.0) renamed so the game does not load it as a mod. It is not stored in this repository.
