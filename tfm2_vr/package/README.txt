TFM2 VR 0.1.1 - Teamfight Manager 2 in a PC VR headset (Steam Workshop version)
================================================================================
(한국어: README_ko.txt)

TFM2 VR shows Teamfight Manager 2 inside a PC VR headset.
It takes the game's own picture and draws it in VR, so the game itself runs as usual.
Matches stand on a table in 3D; the title, management and ban/pick screens appear
on a large curved screen in front of you.


What you need
-------------
- Windows 10/11, 64-bit, Teamfight Manager 2 version 0.7.4
- A PC VR headset with an OpenXR runtime
  (tested: Quest 3 with Air Link. Link cable, Virtual Desktop and SteamVR
  headsets should work too but are not tested. Standalone Quest does not work.)
- Microsoft Visual C++ Redistributable (x64), if the program does not start.


First time
----------
1. Subscribe, start the game, enable TFM2 VR in the Mods menu,
   accept the Code Mod Warning and restart the game.
2. Open this folder:
   Steam Library > right-click Teamfight Manager 2 > Manage > Browse local files.
   That opens steamapps\common\Teamfight Manager2. Go up two folders, then into
   workshop\content\3009300\<this item's id>\
3. (Optional) Run Make_Desktop_Shortcut.bat to get a "TFM2 VR" shortcut
   on your desktop, so you do not need this folder again.


Every time
----------
1. Connect the headset to the PC. The headset's PC app must be the PC's
   default OpenXR runtime (Meta app: Settings > General > OpenXR Runtime,
   SteamVR: Settings > OpenXR).
2. Run TFM2_VR.bat (or the desktop shortcut). The game starts by itself;
   if it is already running, VR attaches to it.
3. To quit, close the game or the black window.


Controls
--------
Point with the controller laser. It works like a mouse.
- Trigger: click a window / grab the table when pointing at nothing
- Grip: grab the table
- Stick: scroll a window / left stick = distance and height, right stick = turn and size
- A / X: re-center      - B / Y: open or close the keyboard
- Grab with both hands and pull apart or turn to resize and rotate.
- Small panel at the bottom left: Curved <-> Flat screen, Re-center, Keyboard.


Language
--------
VR text follows the game's language (Korean or English).
To force it, set lang = en (or ko, auto) in viewer.toml.


Notes
-----
- If you installed the zip version before, delete <game folder>\mods\tfm2_vr_bridge
  so the mod is not loaded twice.
- openxr_loader.dat is the Khronos Group OpenXR loader (openxr_loader.dll,
  Apache License 2.0), renamed so the game does not try to load it as a mod.
- If something goes wrong, please send the newest file in _tmp\logs.
- Unofficial fan mod. Not affiliated with the game's developer.
