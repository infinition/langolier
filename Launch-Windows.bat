@echo off
setlocal
cd /d "%~dp0"
if not exist "src-tauri\target\release\langolier.exe" (
  call Build-Release.bat
  if errorlevel 1 exit /b 1
)
where ollama >nul 2>nul && start "Langolier engine" /min ollama serve
start "Langolier" "src-tauri\target\release\langolier.exe"
