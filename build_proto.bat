@echo off
REM Build script for generating protobuf code
REM This script ensures the correct linker is used by finding and using
REM the Visual Studio Developer Command Prompt

echo Searching for Visual Studio Developer Command Prompt...

set "VS_DEV_CMD="
for /f "usebackq delims=" %%i in (`dir /b /s "C:\Program Files\Microsoft Visual Studio\*\\VC\Auxiliary\Build\vcvars64.bat" 2^>nul`) do (
    set "VS_DEV_CMD=%%i"
    goto :found_vs
)

echo ERROR: Visual Studio Developer Command Prompt not found!
echo Please install Visual Studio with C++ build tools.
pause
exit /b 1

:found_vs
echo Found: %VS_DEV_CMD%
echo.
echo Setting up Visual Studio environment...
call "%VS_DEV_CMD%" >nul 2>&1

echo.
echo ===== Building Client Proto Code =====
cd /d "%~dp0client"
cargo build
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Client build failed!
    cd /d "%~dp0"
    pause
    exit /b 1
)

echo.
echo ===== Building Server Proto Code =====
cd /d "%~dp0server"
cargo build
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Server build failed!
    cd /d "%~dp0"
    pause
    exit /b 1
)

echo.
echo ===== Build Complete! =====
echo Proto files should now be in:
echo - client\src\proto\
echo - server\src\proto\

cd /d "%~dp0"
pause
