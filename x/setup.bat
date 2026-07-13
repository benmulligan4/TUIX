@echo off
REM TUIX Setup Script — runs install, clone, and build in order.
REM 1. install.bat — cargo install for global entries
REM 2. clone.bat — clone local entries to downloads\
REM 3. build.bat — cargo build --release for local entries
REM Logs actions to tuix.log.

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."
set "LOG_FILE=%PROJECT_ROOT%\tuix.log"

for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
echo [%TIMESTAMP%] [SCRIPT] setup.bat started >> "%LOG_FILE%"

echo ========================================
echo   TUIX Setup
echo ========================================
echo.

REM Step 1: Install global entries
echo --- Step 1/3: Installing global entries ---
echo.
call "%SCRIPT_DIR%\install.bat"
echo.

REM Step 2: Clone local entries
echo --- Step 2/3: Cloning local entries ---
echo.
call "%SCRIPT_DIR%\clone.bat"
echo.

REM Step 3: Build local entries
echo --- Step 3/3: Building local entries ---
echo.
call "%SCRIPT_DIR%\build.bat"
echo.

echo ========================================
echo   TUIX Setup Complete
echo ========================================
for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
echo [%TIMESTAMP%] [SCRIPT] setup.bat completed >> "%LOG_FILE%"
