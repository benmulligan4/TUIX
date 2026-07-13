@echo off
REM Install third-party dashboards and applications with source "global" via cargo install.
REM Reads config\dashboards.json and config\apps.json for entries with source "global".
REM Logs actions to tuix.log.

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."

set "DASHBOARDS_JSON=%PROJECT_ROOT%\config\dashboards.json"
set "APPS_JSON=%PROJECT_ROOT%\config\apps.json"
set "LOG_FILE=%PROJECT_ROOT%\tuix.log"

for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
echo [%TIMESTAMP%] [SCRIPT] install.bat started >> "%LOG_FILE%"

echo === TUIX Install Script (Global) ===
echo.

call :install_from_json "%DASHBOARDS_JSON%" "Dashboards"
echo.
call :install_from_json "%APPS_JSON%" "Applications"

echo.
echo === Done ===
for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
echo [%TIMESTAMP%] [SCRIPT] install.bat completed >> "%LOG_FILE%"
goto :eof

:install_from_json
set "json_file=%~1"
set "category=%~2"

if not exist "%json_file%" (
    echo [%category%] Config not found: %json_file% — skipping.
    for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
    echo [!TIMESTAMP!] [ERROR] install.bat: Config not found: %json_file% >> "%LOG_FILE%"
    goto :eof
)

REM Use python to parse JSON and extract entries with source "global" and a cmd
for /f "tokens=1,2 delims=|" %%A in ('python -c "import json; f=open(r'%json_file%'); data=json.load(f); [print(f'{n}|{m[\"cmd\"][0]}') for n,m in data.items() if m.get('source')==='global' and m.get('cmd')]" 2^>nul') do (
    set "name=%%A"
    set "crate_name=%%B"

    echo [%category%] Installing !name! ^(!crate_name!^) via cargo install ...
    cargo install "!crate_name!" >nul 2>&1
    if !errorlevel! equ 0 (
        echo [%category%] !name! installed successfully.
        for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
        echo [!TIMESTAMP!] [SCRIPT] install.bat: Installed !name! ^(!crate_name!^) globally >> "%LOG_FILE%"
    ) else (
        echo [%category%] ERROR: Failed to install !name! ^(!crate_name!^)
        for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
        echo [!TIMESTAMP!] [ERROR] install.bat: Failed to cargo install !crate_name! >> "%LOG_FILE%"
    )
)
goto :eof
