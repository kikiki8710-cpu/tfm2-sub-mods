TFM2 Battle Lab v{VER}  (for Teamfight Manager 2 {GV} only)
==================================================

An unofficial tool that runs only the match (battle) part of the game, without starting the game.
- Re-watch the replays in your save
- Change champions, stats, items, tactics and the seed, then run the match
- Try champion skills on dummies (mod champions too)
- See why the AI chose an action
- Change game rules, champion numbers and item numbers

Battles are computed by loading the code of the game installed on your PC as it is.
Graphics are also taken from the installed game. This package contains no game files.

Language: English by default. Pick 한국어 (Korean) at the top right of the screen to switch (remembered for next time).


1. Requirements
--------------------------------------------------
- Windows 10/11 (64-bit)
- Teamfight Manager 2 {GV} installed through Steam
  (when the game updates, this version stops working — get the new version.)
- At least one save that contains replays
- About 1GB of free space (a match record is 20-80MB; only the latest 10 are kept each time it starts)
- A web browser such as Chrome or Edge


2. Install and run
--------------------------------------------------
1) Extract the zip to any folder. (Not inside the game folder or Program Files)
2) Double-click battle_lab.exe.
   - If "Windows protected your PC" appears: More info -> Run anyway
     (it appears because the program is not signed)
3) A black window opens, then your browser. Graphics are prepared only on the first run (a few seconds).
4) To quit, close the black window.

The game folder is found automatically (on any drive's Steam library). If it is not found, type the game
folder path into the box at the top and press "Use this folder". To write it in a file instead:
  create game_path.txt next to battle_lab.exe and write the game folder path on one line. e.g.
  D:\SteamLibrary\steamapps\common\Teamfight Manager2


3. Screen
--------------------------------------------------
Top     Save / Reload / Mods / 4 tabs / language
Left    Settings of the selected tab
Center  Match view - mouse drag: pan, wheel: zoom
Bottom  Play/Pause, speed (1/10x - 8x), timeline, "Follow selected"
        Tick by tick: the "◀ 1 tick" / "1 tick ▶" buttons or the ← → keys  (-60/+60 = 1 second, Shift+← →)
        Range: tick Attack / Skill 1 / Skill 2 / Ult to show range circles of the selected player
        (the caster on the placement screen). They include level and buffs; a skill reaches when the
        center of an enemy champion is inside the circle (self-targeted skills are not shown).
        Type a number in the "Tick" box and press Enter to jump there. Space = play/pause, Home/End = start/end
        When paused, animations stop at that tick too (60 ticks = 1 second).
Right   Players (click to select), decision reasons, events  (hidden on narrow windows; use more than 1100 pixels)


4. Tabs
--------------------------------------------------
[Replay]
  Pick a "Year" and "League" at the top; the list shows the newest matches 500 at a time ("Show more" below).
  Solo Rank matches are not in the list (the game does not keep them as replays).
  Click a match in the list to re-simulate and show it (usually a few seconds, with a progress bar).
  Check "Record decisions" before clicking to see the chosen goal, action and scores
  under "Decision reasons" on the right (makes bigger files).
  * Matches played with mods, or in an older game version, may play out differently from the record.

[Custom match]
  1) Pick a "Base match". Its players, stats and items become the defaults.
  2) Change champions, and open "Stats & build" to change stats and the 4 target items.
  3) In "Change team tactics", pick only the tactics you want to change.
  4) Enter a seed for a different flow of the match. Leave it empty for the original seed.
     Same settings + same seed = always the same result.
     When comparing, run several matches with different seeds (one match is mostly luck).
  5) "Start match". Settings can be saved under a name and loaded again.

[Skill lab]
  Set the caster and level, the dummies (enemy/ally, HP, level) and the cast order (tick, skill, target), then "Run".
  - 60 ticks = 1 second
  - Skill 2 works from level 3, the ult from level 5.
  - Dummies stand still without AI. HP 0 = full.
  - Position: press "Place on the map" to drag the caster and dummies with the mouse.
    You can also type numbers in the x/y boxes; the distance to the caster is shown (same unit as skill range).
    Units placed in a wall are pushed aside, and units inside enemy tower range get shot (⚠ mark).
  - Turn "Spawn minions" off (default) to test without minions.
  - Cooldowns are reset at each cast. A cast fails if the dummy is out of range.

[Rules]
  Pick game rules / champion stats / items, search by name and edit numbers.
  Edits apply to runs in every tab. "Reset this kind" undoes them.


5. Using mods
--------------------------------------------------
"Mods" lists the mods in the game's mods folder and the Steam Workshop.
Selected mods are remembered and stay on next time. "Turn all off" turns them all off at once.
- Champion files (.data_champion)
    Add/fill : adds champions that are not in the save.
    Replace  : replaces a champion already in the save with the current file content.
    (champions stored in the save win over the file, so choose "Replace" to see your edits)
- Setting override : mods that change items, rules or champion numbers
- dll mods : only those marked "Official API" work. "Legacy" ones do not.
    * A dll is program code. Only enable mods you trust.
    * Mods that only change the screen (UI) make no difference to battles.

Testing a champion you are making:
  1) Put the whole mod folder into this program's devmods folder.
     e.g. devmods\my_mod\champion\my_champ.data_champion
  2) "Reload" -> in "Mods", pick the one marked [dev].
  3) Pick it from the champion list in "Skill lab" or "Custom match".
  4) After editing the file, run again and the change is used right away (no game or new game needed).


6. Limits / differences
--------------------------------------------------
- No sound. Some graphics and effects such as minions may look different from the game.
- Nothing outside matches (draft, seasons, training, ...).
- Mods that store per-save settings may run with default values.
- Mod dlls are not guaranteed to behave exactly as in the game.
- Some texts that come from the simulator itself (decision details, logs) stay in Korean.


7. Troubleshooting
--------------------------------------------------
- "only works with game {GV}" warning -> your game version differs. Get the matching version.
- "The simulator (lab_worker.exe) is missing" -> your antivirus may have removed it. Extract the zip again.
- "This save has no playable replays" -> pick another save at the top, or play a match in the game and save.
- "Failed: ..." -> turn off all mods and try again. If it still fails without mods, send the
                   scenario.txt from the latest folder under runs.
- "simulator crashed 0x..." -> the simulator stopped midway. If it says "out of memory", close the game
                   and other programs and retry. If it keeps happening, send the whole message and your save size.


8. What it does on your PC
--------------------------------------------------
- Saves and the game folder are only read. It only writes inside this program's folder.
  (except files that a dll mod you enabled writes by itself)
- It only listens inside your PC (127.0.0.1) and sends nothing over the internet.
- To remove it, delete the whole folder. Nothing is installed.

Unofficial tool, not affiliated with the developer of Teamfight Manager 2. Use at your own risk.
