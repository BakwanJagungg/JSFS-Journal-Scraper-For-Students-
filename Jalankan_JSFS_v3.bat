@echo off
title JSFS v3.0.0 — Journal Scraper For Students
setlocal

set "EXE_PATH=%~dp0src-tauri\target\release\jsfs.exe"

if exist "%EXE_PATH%" (
    echo Menjalankan JSFS v3.0.0 (Rust + Tauri v2 Native)...
    start "" "%EXE_PATH%"
    exit /b 0
)

echo ====================================================================
echo   [INFO] File biner jsfs.exe belum di-build di folder release.
echo ====================================================================
echo.
echo Silakan jalankan perintah berikut sekali saja di folder ini:
echo   1. npm install
echo   2. npm run build   (atau: npx tauri build --no-bundle)
echo.
echo Atau jalankan langsung dalam mode dev:
echo   npm run dev
echo.
pause
