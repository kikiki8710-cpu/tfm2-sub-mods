# Inhibitors & Super Minions (tfm2_inhibitor)

League of Legends-style inhibitors for Teamfight Manager 2 5v5 matches.

## What it does
- Every lane gets an inhibitor behind its second tower (six in total). Inhibitors never attack.
- An inhibitor can only be damaged after both towers of its lane are destroyed. Its health equals a nexus tower's.
- The two nexus towers can only be damaged while at least one of their team's inhibitors is down.
- Once both nexus towers are destroyed, the nexus can be attacked, even if inhibitors are alive again.
- While an inhibitor is down, every enemy minion wave in that lane brings one extra super minion
  (a big armored knight: 2.5x attack, 4x health, +40 defense / magic resist over a melee minion).
- Destroyed inhibitors come back after 5 minutes. That lane then stops spawning super minions.
- The AI sieges inhibitors after the second tower and goes for the nexus once both nexus towers are down.

## Install
1. Unzip and put the `tfm2_inhibitor` folder into the game's `mods` folder.
2. Start the game, enable the mod in the Mod Manager (Mods on the title screen), confirm the Code Mod Warning and restart the game.

## Settings
`config/inhibitor.cfg` (plain text, comments in English and Korean). Restart the game after editing.
Delete the file to get the defaults back. Main settings:
- `lanes` = all, or any of top, mid, bot
- `respawn_sec` = 300 (0 = never respawn)
- `hp_pct` = 100 (% of a nexus tower)
- `lock_twin` / `lol_nexus` = 1 (0 = vanilla rules)
- `super_minion` = 1, `super_attack_pct` = 150, `super_hp_pct` = 300, `super_defence` = 40, `super_magic_resist` = 40

## Notes
- Built for game 0.7.4. Other game versions disable the mod automatically.
- Only 5v5 matches change. 2v2, 3v3 and the tutorial stay vanilla.
- Works together with Elemental & Elder Serpen.
- Code mod (native DLL, Windows only).
