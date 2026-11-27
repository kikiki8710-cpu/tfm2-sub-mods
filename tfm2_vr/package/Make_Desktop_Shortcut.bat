@echo off
rem Creates a "TFM2 VR" shortcut on the desktop that runs TFM2_VR.bat in this folder.
powershell -NoProfile -ExecutionPolicy Bypass -Command "$d='%~dp0'; $s=(New-Object -ComObject WScript.Shell).CreateShortcut([Environment]::GetFolderPath('Desktop')+'\TFM2 VR.lnk'); $s.TargetPath=$d+'TFM2_VR.bat'; $s.WorkingDirectory=$d; $s.IconLocation=$d+'tfm2_vr_viewer.exe,0'; $s.Save()"
if errorlevel 1 (echo Could not create the shortcut.) else (echo Desktop shortcut "TFM2 VR" created.)
pause
