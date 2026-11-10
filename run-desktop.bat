@echo off
echo ==============================================
echo   chatw - Launching Standalone Desktop App
echo ==============================================

set "PATH=E:\.cargo\bin;C:\Users\%USERNAME%\.cargo\bin;%PATH%"

echo 1. Ensuring PostgreSQL is running on port 55432...
netstat -ano | findstr 55432 >nul
if %errorlevel% neq 0 (
    echo Starting PostgreSQL service...
    docker start chatw-postgres-1 >nul 2>&1
    if %errorlevel% neq 0 (
        start /B wsl sh "%~dp0scripts\run_daemon.sh"
    )
    timeout /t 2 /nobreak >nul
)

echo 2. Ensuring Rust Backend is running on port 8085...
netstat -ano | findstr 8085 >nul
if %errorlevel% neq 0 (
    echo Starting Rust Backend (Port 8085)...
    start "chatw-backend" cmd /k "cd /d %~dp0backend && set DATABASE_URL=postgres://chatw:chatw_dev_password@127.0.0.1:55432/chatw&& cargo run"
    timeout /t 2 /nobreak >nul
)

echo 3. Launching Standalone Desktop App...
start "" "%~dp0dist-desktop\chatw.exe"
