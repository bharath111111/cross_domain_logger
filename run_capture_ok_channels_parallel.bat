@echo off
setlocal EnableExtensions
cd /d "%~dp0" || exit /b 1
setlocal EnableDelayedExpansion

set "CAPTURE_MS=%~1"
if not defined CAPTURE_MS set "CAPTURE_MS=60000"

set "APP_NAME=CANoe"
set "IFACE_VER=4"
set "CHANNELS=1 2 3 4 5 6 7 8 9 10 11"

if not exist "cross_domain_logger_windows.exe" (
  echo ERROR: cross_domain_logger_windows.exe not found in %~dp0
  echo Keep this BAT in the same folder as the EXE.
  pause
  exit /b 1
)

if not exist "vxlapi64.dll" (
  echo ERROR: vxlapi64.dll not found in %~dp0
  echo Keep this DLL in the same folder as the EXE.
  pause
  exit /b 1
)

echo Starting parallel CAN capture for channels: %CHANNELS%
echo Duration: %CAPTURE_MS% ms
echo App: %APP_NAME%, Interface Version: %IFACE_VER%
echo.

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
  start "CAN_%%C" /min cmd /c ""%~dp0cross_domain_logger_windows.exe" --test-can --can-listen --can-channel !APP_CH! --can-app %APP_NAME% --can-iface-version %IFACE_VER% --can-duration-ms %CAPTURE_MS% --can-log-format asc --can-log-file "%~dp0!NET_NAME!.asc" 1> "%~dp0!NET_NAME!_console.log" 2>&1"
)

set /a CAPTURE_SEC=(%CAPTURE_MS% + 999) / 1000 + 5
echo.
echo Waiting %CAPTURE_SEC% seconds for captures to finish...
timeout /t %CAPTURE_SEC% /nobreak >nul

echo.
echo Capture complete. Generated ASC files:
dir /b *.asc 2>nul
echo.
echo Console logs:
dir /b *_console.log 2>nul
echo.
echo Done. Press any key to close.
pause >nul
