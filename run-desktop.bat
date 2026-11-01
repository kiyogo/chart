@echo off
echo ==============================================
echo   chatw - Launching Desktop App (Tauri v2)
echo ==============================================

set "PATH=E:\.cargo\bin;C:\Users\%USERNAME%\.cargo\bin;%PATH%"

cd /d "%~dp0frontend"
call npm run tauri:dev
pause
