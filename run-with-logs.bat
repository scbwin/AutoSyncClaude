@echo off
echo ================================================
echo Claude Sync GUI - Debug Log Launcher
echo ================================================
echo.
echo Starting with RUST_LOG=debug...
echo Log file: %~dp0debug-log.txt
echo.
echo Press Ctrl+C to stop logging
echo ================================================
echo.

set RUST_LOG=debug
set LOG_FILE=%~dp0debug-log.txt

REM 替换为你的实际安装路径
set APP_PATH=C:\Program Files\你的应用路径\claude-sync-gui.exe

REM 如果找不到，尝试常见位置
if not exist "%APP_PATH%" (
    if exist "%LOCALAPPDATA%\Programs\claude-sync-gui\claude-sync-gui.exe" (
        set APP_PATH=%LOCALAPPDATA%\Programs\claude-sync-gui\claude-sync-gui.exe
    ) else if exist "%USERPROFILE%\AppData\Local\Programs\claude-sync-gui\claude-sync-gui.exe" (
        set APP_PATH=%USERPROFILE%\AppData\Local\Programs\claude-sync-gui\claude-sync-gui.exe
    )
)

echo App Path: %APP_PATH%
echo.

REM 清空旧日志
echo === Log started at %date% %time% > "%LOG_FILE%"
echo Rust Log Level: %RUST_LOG% >> "%LOG_FILE%"
echo. >> "%LOG_FILE%"

REM 启动应用（日志会写入文件）
start "" "%APP_PATH%"

echo.
echo Application started. Check %LOG_FILE% for logs.
echo Note: GUI apps may not output to file. Try the console version below.
echo.
pause
