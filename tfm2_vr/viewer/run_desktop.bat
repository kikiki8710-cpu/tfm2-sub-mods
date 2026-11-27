@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"
"target\release\tfm2_vr_viewer.exe" --desktop
set "viewer_exit=%ERRORLEVEL%"
echo 종료 코드: %viewer_exit%
pause
exit /b %viewer_exit%
