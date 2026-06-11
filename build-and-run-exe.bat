@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\run-windows-test-shell.ps1"
if errorlevel 1 (
  pause
  exit /b %errorlevel%
)
