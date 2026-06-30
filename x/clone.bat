@echo off
REM Clone third-party dashboards and applications that aren't already installed.
REM Reads config\dashboards.json and config\apps.json for entries with "repository" field.

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."

set "DASHBOARDS_JSON=%PROJECT_ROOT%\config\dashboards.json"
set "APPS_JSON=%PROJECT_ROOT%\config\apps.json"
set "DASHBOARDS_INSTALLED=%PROJECT_ROOT%\src\dashboards\installed"
set "APPS_INSTALLED=%PROJECT_ROOT%\src\applications\installed"

REM Ensure installed directories exist
if not exist "%DASHBOARDS_INSTALLED%" mkdir "%DASHBOARDS_INSTALLED%"
if not exist "%APPS_INSTALLED%" mkdir "%APPS_INSTALLED%"

echo === TUIX Clone Script ===
echo.

call :clone_from_json "%DASHBOARDS_JSON%" "%DASHBOARDS_INSTALLED%" "Dashboards"
echo.
call :clone_from_json "%APPS_JSON%" "%APPS_INSTALLED%" "Applications"

echo.
echo === Done ===
goto :eof

:clone_from_json
set "json_file=%~1"
set "install_dir=%~2"
set "category=%~3"

if not exist "%json_file%" (
    echo [%category%] Config not found: %json_file% — skipping.
    goto :eof
)

REM Use python to parse JSON and extract repository entries
for /f "tokens=1,2 delims=|" %%A in ('python -c "import json; f=open(r'%json_file%'); data=json.load(f); [print(f'{n}|{m[\"repository\"]}') for n,m in data.items() if 'repository' in m]" 2^>nul') do (
    set "name=%%A"
    set "repo=%%B"
    set "target=%install_dir%\!name!"

    if exist "!target!" (
        echo [%category%] !name! already installed — skipping.
    ) else (
        echo [%category%] Cloning !name! from !repo! ...
        git clone "!repo!" "!target!"
        REM Remove .github directory from cloned repo
        if exist "!target!\.github" rmdir /s /q "!target!\.github"
        echo [%category%] !name! installed successfully.
    )
)
goto :eof
