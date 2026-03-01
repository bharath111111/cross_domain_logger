@echo off
setlocal EnableExtensions
cd /d "%~dp0"

set "RUN_LOG=run_capture_all_connected_auto.log"
echo ==== START %date% %time% ==== > "%RUN_LOG%"
echo Working dir: %cd% >> "%RUN_LOG%"

set "CAPTURE_MS=%~1"
if not defined CAPTURE_MS set "CAPTURE_MS=60000"

set "APP_NAME=CANoe"
set "IFACE_VER=4"
set "OUT_DIR=all_connected_logs"
set "NETWORKS=FD_CANW FD_CAN5 FD_CAN9 FD_CAN13 FD_CAN14 FD_CAN15 FD_CAN17 FD_CAN18 FD_CAN20 FD_CAN21 HS_CAN1"

if not exist "cross_domain_logger_windows.exe" (
  echo ERROR: cross_domain_logger_windows.exe not found in %~dp0
  echo ERROR: EXE missing >> "%RUN_LOG%"
  goto :end_error
)

if not exist "vxlapi64.dll" (
  echo ERROR: vxlapi64.dll not found in %~dp0
  echo ERROR: DLL missing >> "%RUN_LOG%"
  goto :end_error
)

echo Auto-detecting usable channels and capturing all connected ones...
echo Duration: %CAPTURE_MS% ms
echo Output folder: %OUT_DIR%
echo.
echo Launch command with duration %CAPTURE_MS% >> "%RUN_LOG%"

cross_domain_logger_windows.exe --test-can --can-listen-all --can-max-channels 64 --can-app %APP_NAME% --can-iface-version %IFACE_VER% --can-duration-ms %CAPTURE_MS% --can-log-format asc --can-output-dir %OUT_DIR% >> "%RUN_LOG%" 2>&1
set "ERR=%ERRORLEVEL%"

if not "%ERR%"=="0" (
  echo.
  echo Capture command failed with exit code %ERR%.
  echo See %RUN_LOG% for details.
  goto :end_error
)

echo.
echo Capture completed. Files:
for %%N in (%NETWORKS%) do (
  if exist "%OUT_DIR%\%%N.asc" echo %%N.asc
)
echo Capture completed successfully >> "%RUN_LOG%"
for %%N in (%NETWORKS%) do (
  if exist "%OUT_DIR%\%%N.asc" echo %%N.asc >> "%RUN_LOG%"
)
echo.
goto :end_ok

:end_error
echo ==== END WITH ERROR %date% %time% ==== >> "%RUN_LOG%"
if defined NO_PAUSE exit /b 1
echo Press any key to close.
pause >nul
exit /b 1

:end_ok
echo ==== END OK %date% %time% ==== >> "%RUN_LOG%"
if defined NO_PAUSE exit /b 0
echo Press any key to close.
pause >nul
