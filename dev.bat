@echo off
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cd /d "%~dp0"
echo Iniciando DuelCraft em modo dev (%cd%)...
npx.cmd tauri dev
pause
