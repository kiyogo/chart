@echo off
echo ==============================================
echo   chatw - Starting Full Stack (DB + Backend + Desktop)
echo ==============================================

set "PATH=E:\.cargo\bin;C:\Users\%USERNAME%\.cargo\bin;%PATH%"

echo 1. Ensuring PostgreSQL is running...
docker-compose up -d postgres

echo 2. Starting Rust Backend (Port 8082)...
start "chatw-backend" cmd /k "cd /d %~dp0backend && cargo run"

timeout /t 3 /nobreak >nul

echo 3. Launching Desktop App...
cd /d "%~dp0frontend"
npm run tauri:dev
