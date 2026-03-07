@echo off
setlocal EnableExtensions

set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%.."

if exist "%SCRIPT_DIR%probe_testpc_apis.py" (
  set "PY_SCRIPT=%SCRIPT_DIR%probe_testpc_apis.py"
) else (
  set "PY_SCRIPT=%ROOT_DIR%\scripts\probe_testpc_apis.py"
)

set "OUT_FILE=%~1"
if not defined OUT_FILE set "OUT_FILE=CAN_LOGS\testpc_api_probe.txt"

if not exist "%PY_SCRIPT%" (
  echo ERROR: probe script not found: %PY_SCRIPT%
  if defined NO_PAUSE exit /b 1
  pause
  exit /b 1
)

py -3 "%PY_SCRIPT%" --out "%OUT_FILE%"
set "ERR=%ERRORLEVEL%"

if not "%ERR%"=="0" (
  echo ERROR: API probe failed with exit code %ERR%.
  if defined NO_PAUSE exit /b %ERR%
  pause
  exit /b %ERR%
)

echo Probe report generated: %OUT_FILE%
if defined NO_PAUSE exit /b 0
pause
exit /b 0
