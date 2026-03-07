@echo off
setlocal EnableExtensions

set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%.."

if exist "%SCRIPT_DIR%make_triage_pack.py" (
  set "PY_SCRIPT=%SCRIPT_DIR%make_triage_pack.py"
) else (
  set "PY_SCRIPT=%ROOT_DIR%\scripts\make_triage_pack.py"
)

if not exist "%PY_SCRIPT%" (
  echo ERROR: make_triage_pack.py not found: %PY_SCRIPT%
  if defined NO_PAUSE exit /b 1
  pause
  exit /b 1
)

set "NOTE=%~1"
if not defined NOTE set "NOTE=Manual ControlDesk CAN capture + Ethernet capture for developer triage"

py -3 "%PY_SCRIPT%" --source-dir CAN_LOGS --output-dir dist --note "%NOTE%"
set "ERR=%ERRORLEVEL%"

if not "%ERR%"=="0" (
  echo ERROR: Triage pack creation failed with exit code %ERR%.
  if defined NO_PAUSE exit /b %ERR%
  pause
  exit /b %ERR%
)

echo Triage package created under dist.
if defined NO_PAUSE exit /b 0
pause
exit /b 0
