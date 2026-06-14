@echo off
setlocal
cd /d "%~dp0"
echo Building LocalAreaInterconnection desktop executable...
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-windows-test-shell.ps1"
if errorlevel 1 (
  echo.
  echo Build failed. Check the error above.
  pause
  exit /b %errorlevel%
)
echo.
echo Build complete:
echo   %~dp0dist\LocalAreaInterconnection.exe
echo.
pause
