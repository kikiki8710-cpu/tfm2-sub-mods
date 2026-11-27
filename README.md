# SUB's Teamfight Manager 2 mods

Source code and releases of my Teamfight Manager 2 mods and tools, one folder per item.
The same builds are on the Steam Workshop; subscribing there is the easiest way to install them.

Current game version: **0.7.4**. Every release is built for one game version (see [Releases](../../releases)).

## Items

| Folder | Name | Version | Workshop | What it does |
|---|---|---|---|---|
| [`tfm2_level_cap`](tfm2_level_cap) | Champion Level Cap 18 | 2.2.0 | [3846485458](https://steamcommunity.com/sharedfiles/filedetails/?id=3846485458) | Champions keep leveling past 12 in matches (cap configurable). |
| [`tfm2_elemental_serpen`](tfm2_elemental_serpen) | Elemental & Elder Serpen | 0.6.1 | [3846485884](https://steamcommunity.com/sharedfiles/filedetails/?id=3846485884) | LoL-style elemental Serpens with stacking team buffs and the Elder Serpen. |
| [`tfm2_inhibitor`](tfm2_inhibitor) | Inhibitors & Super Minions | 0.1.7 | [3846486519](https://steamcommunity.com/sharedfiles/filedetails/?id=3846486519) | Inhibitors behind each lane's second tower; super minions while one is down. |
| [`tfm2_champ_pos_lock`](tfm2_champ_pos_lock) | Champion Position Lock | 0.7.8 | [3846486017](https://steamcommunity.com/sharedfiles/filedetails/?id=3846486017) | Choose which champions may play each position, for every team. |
| [`tfm2_detail_tactics`](tfm2_detail_tactics) | Detail Tactics | 0.2.9 | [3846486412](https://steamcommunity.com/sharedfiles/filedetails/?id=3846486412) | 63 AI decision thresholds per position or team-wide. |
| [`tfm2_transfer_fix`](tfm2_transfer_fix) | Transfer & Salary Fix | 0.2.2 | [3846486155](https://steamcommunity.com/sharedfiles/filedetails/?id=3846486155) | Removes the AI's 2.59x asking price floor and caps runaway salaries. |
| [`tfm2_champion_exclude`](tfm2_champion_exclude) | Champion Additions Control | 0.6.2 | [3846485634](https://steamcommunity.com/sharedfiles/filedetails/?id=3846485634) | Keep chosen (or all) unreleased champions out of season patches. |
| [`tfm2_comptest_unlock`](tfm2_comptest_unlock) | Composition Test Unlocked | 0.6.0 | [3846485755](https://steamcommunity.com/sharedfiles/filedetails/?id=3846485755) | No daily limit or stamina cost, up to 10 tests at once. |
| [`banpick_view_plus`](banpick_view_plus) | Ban/Pick View Plus | 0.6.4 | [3846486638](https://steamcommunity.com/sharedfiles/filedetails/?id=3846486638) | Bigger ban/pick screen with tiers, stat radar and splash-card slots. |
| [`daram2_view_plus`](daram2_view_plus) | View Plus (Integrated) | 1.0.0 | [3846486746](https://steamcommunity.com/sharedfiles/filedetails/?id=3846486746) | Roster / Staff / Recruitment / Training / Facilities / Custom Tiers in one mod. |
| [`tfm2_mod_order`](tfm2_mod_order) | Mod List Reorder | 0.2.2 | [3846486307](https://steamcommunity.com/sharedfiles/filedetails/?id=3846486307) | Reorder the Mod Manager list with Ctrl + Up / Down. |
| [`tfm2_vr`](tfm2_vr) | TFM2 VR | 0.1.1 | [3846488476](https://steamcommunity.com/sharedfiles/filedetails/?id=3846488476) | Play in a PC VR headset (bridge mod + OpenXR viewer). |
| [`tfm2_battle_lab`](tfm2_battle_lab) | TFM2 Battle Lab | 0.1.3 | [3846572460](https://steamcommunity.com/sharedfiles/filedetails/?id=3846572460) | Standalone program: replay, simulate and test matches outside the game. |

Shared code: [`common`](common) (log folder helper) and [`ui_kit`](ui_kit) (UI helpers), pulled in with `#[path = ...]`.

## Install

- **Workshop**: subscribe, enable the mod in the Mod Manager (**Mods** on the title screen), confirm the Code Mod Warning and restart the game.
- **Manual**: download the zip from [Releases](../../releases) and extract it into `<game folder>\mods\` so that you get `mods\<mod_id>\<mod_id>.dll` and `mod.mod_info`. Use only one of the two (Workshop or manual) for the same mod.

Mods only load on the game version they were built for; other versions disable them automatically.

## Build

The game mods use the game's official stable mod API.

1. Install Rust from <https://rustup.rs>.
2. Copy `<game folder>\mod-sdk-stable\mod-api-stable` into the root of this repository (it is not included here; it ships with the game).
3. In a mod folder: `cargo build --release`. The dll is in `target\release\`.
4. Copy the dll as `<mod_id>.dll` together with `mod.mod_info` (and `mod.override_info` / data files if the folder has them) into `<game folder>\mods\<mod_id>\`.

Keep `opt-level = 1` from each `Cargo.toml`: higher levels make some hooks use too much stack.

`tfm2_vr` and `tfm2_battle_lab` have their own notes in their folders.

## Notes

- Many mods patch or hook the game's own code at fixed addresses, so they are updated for each game version. Addresses in the source are for the version in the release they belong to.
- Code comments are mostly in Korean.
- Made with the help of AI (LLM) tools.
- Unofficial; not affiliated with the developer of Teamfight Manager 2.

## Credits

- `banpick_view_plus` and `daram2_view_plus` are based on the original View Plus mods by **daram2 (다람이)**, ported to the stable mod API and maintained by SUB.

## License

[MIT](LICENSE).
