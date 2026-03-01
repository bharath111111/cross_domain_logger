@echo off
setlocal EnableExtensions
set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%.."

if exist "%SCRIPT_DIR%cross_domain_logger_windows.exe" (
  set "EXE=%SCRIPT_DIR%cross_domain_logger_windows.exe"
  set "DLL=%SCRIPT_DIR%vxlapi64.dll"
  set "WORK_DIR=%SCRIPT_DIR%"
) else (
  set "EXE=%ROOT_DIR%\target\release\cross_domain_logger.exe"
  set "DLL=%ROOT_DIR%\vxlapi64.dll"
  set "WORK_DIR=%ROOT_DIR%\"
)

cd /d "%WORK_DIR%" || exit /b 1
setlocal EnableDelayedExpansion

set "CAPTURE_MS=%~1"
if not defined CAPTURE_MS set "CAPTURE_MS=60000"

set "APP_NAME=CANoe"
set "IFACE_VER=4"
set "CHANNELS=1 2 3 4 5 6 7 8 9 10 11"
set "OUT_DIR=CAN_LOGS"

if not exist "%EXE%" (
  echo ERROR: Executable not found: %EXE%
  pause
  exit /b 1
)

if not exist "%DLL%" (
  echo ERROR: DLL not found: %DLL%
  pause
  exit /b 1
)

echo Starting parallel CAN capture for channels: %CHANNELS%
echo Duration: %CAPTURE_MS% ms
echo App: %APP_NAME%, Interface Version: %IFACE_VER%
echo Output folder: %OUT_DIR%
echo.

if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"

for %%C in (%CHANNELS%) do (
  set /a APP_CH=%%C-1
  set "NET_NAME=channel%%C"
  if %%C==1 set "NET_NAME=FD_CANW"
  if %%C==2 set "NET_NAME=FD_CAN5"
  if %%C==3 set "NET_NAME=FD_CAN9"
  if %%C==4 set "NET_NAME=FD_CAN13"
  if %%C==5 set "NET_NAME=FD_CAN14"
  if %%C==6 set "NET_NAME=FD_CAN15"
  if %%C==7 set "NET_NAME=FD_CAN17"
  if %%C==8 set "NET_NAME=FD_CAN18"
  if %%C==9 set "NET_NAME=FD_CAN20"
  if %%C==10 set "NET_NAME=FD_CAN21"
  if %%C==11 set "NET_NAME=HS_CAN1"
  echo [START] Channel %%C (appCh !APP_CH!)
  start "CAN_%%C" /min cmd /c ""%EXE%" --test-can --can-listen --can-channel !APP_CH! --can-app %APP_NAME% --can-iface-version %IFACE_VER% --can-duration-ms %CAPTURE_MS% --can-log-format asc --can-log-file "%OUT_DIR%\!NET_NAME!.asc" 1> "%OUT_DIR%\!NET_NAME!_console.log" 2>&1"
)

set /a CAPTURE_SEC=(%CAPTURE_MS% + 999) / 1000 + 5
echo.
echo Waiting %CAPTURE_SEC% seconds for captures to finish...
timeout /t %CAPTURE_SEC% /nobreak >nul

echo.
echo Capture complete. Generated ASC files:
dir /b "%OUT_DIR%\*.asc" 2>nul
echo.
echo Console logs:
dir /b "%OUT_DIR%\*_console.log" 2>nul
echo.
echo Done. Press any key to close.
pause >nul
