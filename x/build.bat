@echo off
REM Build locally cloned dashboards and applications (source "local").
REM Looks for Cargo.toml in downloads\dashboards\{name} and downloads\applications\{name}.
REM Runs cargo build --release for each.
REM Logs actions to tuix.log.

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."

set "DASHBOARDS_JSON=%PROJECT_ROOT%\config\dashboards.json"
set "APPS_JSON=%PROJECT_ROOT%\config\apps.json"
set "DASHBOARDS_DIR=%PROJECT_ROOT%\downloads\dashboards"
set "APPS_DIR=%PROJECT_ROOT%\downloads\applications"
set "LOG_FILE=%PROJECT_ROOT%\tuix.log"

for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
echo [%TIMESTAMP%] [SCRIPT] build.bat started >> "%LOG_FILE%"

echo === TUIX Build Script (Local) ===
echo.

call :build_from_json "%DASHBOARDS_JSON%" "%DASHBOARDS_DIR%" "Dashboards"
echo.
call :build_from_json "%APPS_JSON%" "%APPS_DIR%" "Applications"

echo.
echo === Done ===
for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
echo [%TIMESTAMP%] [SCRIPT] build.bat completed >> "%LOG_FILE%"
goto :eof

:build_from_json
set "json_file=%~1"
set "downloads_dir=%~2"
set "category=%~3"

if not exist "%json_file%" (
    echo [%category%] Config not found: %json_file% — skipping.
    for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
    echo [!TIMESTAMP!] [ERROR] build.bat: Config not found: %json_file% >> "%LOG_FILE%"
    goto :eof
)

REM Use python to parse JSON and extract entries with source "local"
for /f "tokens=*" %%A in ('python -c "import json; f=open(r'%json_file%'); data=json.load(f); [print(n) for n,m in data.items() if m.get('source')==='local']" 2^>nul') do (
    set "name=%%A"
    set "target=%downloads_dir%\!name!"
    set "cargo_toml=!target!\Cargo.toml"

    if not exist "!target!" (
        echo [%category%] !name! not cloned yet — run clone.bat first. Skipping.
        for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
        echo [!TIMESTAMP!] [ERROR] build.bat: !name! not found at !target! >> "%LOG_FILE%"
    ) else if not exist "!cargo_toml!" (
        echo [%category%] !name! has no Cargo.toml — skipping.
        for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
        echo [!TIMESTAMP!] [ERROR] build.bat: No Cargo.toml for !name! >> "%LOG_FILE%"
    ) else (
        echo [%category%] Building !name! ...
        cargo build --release --manifest-path "!cargo_toml!"
        if !errorlevel! equ 0 (
            echo [%category%] !name! built successfully.
            for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
            echo [!TIMESTAMP!] [SCRIPT] build.bat: Built !name! ^(release^) >> "%LOG_FILE%"
        ) else (
            echo [%category%] ERROR: Failed to build !name!
            for /f "tokens=1-2 delims= " %%a in ('powershell -command "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'"') do set "TIMESTAMP=%%a %%b"
            echo [!TIMESTAMP!] [ERROR] build.bat: Failed to build !name! >> "%LOG_FILE%"
        )
    )
)
goto :eof
