@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"
if not exist "target\release\tfm2_vr_viewer.exe" (
  echo 실행 파일이 없습니다. VIEWER_README.md의 빌드 방법을 확인하세요.
  pause
  exit /b 1
)
echo 헤드셋을 착용하고 PC Link를 준비하세요.
echo 00:30 일시정지로 시작합니다. 자동 종료는 없습니다.
echo 창 닫기 또는 Ctrl+C로 종료합니다.
"target\release\tfm2_vr_viewer.exe" --session
set "viewer_exit=%ERRORLEVEL%"
echo.
echo 종료 코드: %viewer_exit%
echo 로그: ..\_tmp\logs\native_viewer_*.log
pause
exit /b %viewer_exit%
