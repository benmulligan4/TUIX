@echo off
REM Clone third-party dashboards and applications with source "local".
REM Clones to downloads\dashboards\{name} and downloads\applications\{name}.
REM Excludes .git and .github directories from cloned repos.
REM Logs actions to tuix.log.

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."

set "DASHBOARDS_JSON=%PROJECT_ROOT%\config\dashboards.json"
set "APPS_JSON=%PROJECT_ROOT%\config\apps.json"
set "DASHBOARDS_DIR=%PROJECT_ROOT%\downloads\dashboards"
set "APPS_DIR=%PROJECT_ROOT%\downloads\applications"
set "LOG_FILE=%PROJECT_ROOT%\tuix.log"

REM Ensure download directories exist
if not exist "%DASHBOARDS_DIR%" mkdir "%DASHBOARDS_DIR%"
if not exist "%APPS_DIR%" mkdir "%APPS_DIR%"

REM Log start
for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
echo [%TIMESTAMP%] [SCRIPT] clone.bat started >> "%LOG_FILE%"

echo === TUIX Clone Script ===
echo.

call :clone_from_json "%DASHBOARDS_JSON%" "%DASHBOARDS_DIR%" "Dashboards"
echo.
call :clone_from_json "%APPS_JSON%" "%APPS_DIR%" "Applications"

echo.
echo === Done ===
for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
echo [%TIMESTAMP%] [SCRIPT] clone.bat completed >> "%LOG_FILE%"
goto :eof

:clone_from_json
set "json_file=%~1"
set "install_dir=%~2"
set "category=%~3"

if not exist "%json_file%" (
    echo [%category%] Config not found: %json_file% — skipping.
    for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
    echo [!TIMESTAMP!] [ERROR] clone.bat: Config not found: %json_file% >> "%LOG_FILE%"
    goto :eof
)

REM Use python to parse JSON and extract entries with source "local" and a repository
for /f "tokens=1,2 delims=|" %%A in ('python -c "import json; f=open(r'%json_file%'); data=json.load(f); [print(f'{n}|{m[\"repository\"]}') for n,m in data.items() if m.get('source')==='local' and 'repository' in m]" 2^>nul') do (
    set "name=%%A"
    set "repo=%%B"
    set "target=%install_dir%\!name!"

    if exist "!target!" (
        echo [%category%] !name! already cloned — skipping.
        for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
        echo [!TIMESTAMP!] [SCRIPT] clone.bat: !name! already exists, skipped >> "%LOG_FILE%"
    ) else (
        echo [%category%] Cloning !name! from !repo! ...
        git clone "!repo!" "!target!" >nul 2>&1
        if !errorlevel! equ 0 (
            REM Remove .git and .github directories
            if exist "!target!\.git" rmdir /s /q "!target!\.git"
            if exist "!target!\.github" rmdir /s /q "!target!\.github"
            echo [%category%] !name! cloned successfully.
            for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
            echo [!TIMESTAMP!] [SCRIPT] clone.bat: Cloned !name! from !repo! >> "%LOG_FILE%"
        ) else (
            echo [%category%] ERROR: Failed to clone !name! from !repo!
            for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
            echo [!TIMESTAMP!] [ERROR] clone.bat: Failed to clone !name! from !repo! >> "%LOG_FILE%"
        )
    )
)
goto :eof
