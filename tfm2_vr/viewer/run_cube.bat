@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"
if not exist "target\release\tfm2_vr_cube.exe" (
  echo 실행 파일이 없습니다. README.md의 빌드 방법을 확인하세요.
  pause
  exit /b 1
)
echo 헤드셋을 착용하고 PC Link를 준비한 뒤 실행하세요.
echo 큐브 표시 후 60초에 종료하며 Ctrl+C로 종료할 수 있습니다.
"target\release\tfm2_vr_cube.exe" --session
set "cube_exit=%ERRORLEVEL%"
echo.
echo 종료 코드: %cube_exit%
echo 로그는 ..\_tmp\logs\native_cube_*.log 에 있습니다.
pause
exit /b %cube_exit%
