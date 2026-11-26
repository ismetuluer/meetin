@echo off
chcp 65001 >nul
title Meetin
cd /d "%~dp0"
echo Meetin baslatiliyor...
call npm run tauri dev
if errorlevel 1 (
  echo.
  echo Meetin baslatilirken bir hata olustu.
  pause
)
