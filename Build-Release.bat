@echo off
setlocal
cd /d "%~dp0"
where npm >nul 2>nul || (echo Install Node.js LTS first. & pause & exit /b 1)
where cargo >nul 2>nul || (echo Install Rust with the MSVC toolchain first. & pause & exit /b 1)
call npm ci
if errorlevel 1 goto failed
call npm run release -- --bundles nsis
if errorlevel 1 goto failed
echo.
echo Release ready: src-tauri\target\release\langolier.exe
echo Installer: src-tauri\target\release\bundle\nsis
pause
exit /b 0
:failed
echo Build failed. See the error above and docs\DEPLOYMENT.md.
pause
exit /b 1
