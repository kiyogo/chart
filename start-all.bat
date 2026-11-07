@echo off
echo ==============================================
echo   chatw - Starting Full Stack (DB + Backend + Desktop)
echo ==============================================

set "PATH=E:\.cargo\bin;C:\Users\%USERNAME%\.cargo\bin;%PATH%"

echo 1. Ensuring PostgreSQL is running on port 55432...
netstat -ano | findstr 55432 >nul
if %errorlevel% neq 0 (
    echo Starting PostgreSQL service via WSL daemon...
    start /B wsl sh "%~dp0scripts\run_daemon.sh"
    timeout /t 2 /nobreak >nul
) else (
    echo PostgreSQL is already running on port 55432.
)

echo 2. Starting Rust Backend (Port 8082)...
start "chatw-backend" cmd /k "cd /d %~dp0backend && set DATABASE_URL=postgres://chatw:chatw_dev_password@127.0.0.1:55432/chatw&& cargo run"

timeout /t 3 /nobreak >nul

echo 3. Launching Standalone Desktop App...
start "" "%~dp0dist-desktop\chatw.exe"
