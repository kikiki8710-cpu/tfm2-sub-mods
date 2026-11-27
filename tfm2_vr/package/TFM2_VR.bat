@echo off
cd /d "%~dp0"
echo Put on the headset (PC Link on). The game starts by itself.
echo Close this window or the game to finish.
tfm2_vr_viewer.exe --play
if errorlevel 1 pause
