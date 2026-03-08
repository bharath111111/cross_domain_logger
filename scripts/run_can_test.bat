@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%.."

if exist "%SCRIPT_DIR%cross_domain_logger_windows.exe" (
	set "EXE=%SCRIPT_DIR%cross_domain_logger_windows.exe"
	set "OUT_DIR=%SCRIPT_DIR%"
) else (
	set "EXE=%ROOT_DIR%\target\release\cross_domain_logger.exe"
	set "OUT_DIR=%ROOT_DIR%\"
)

cd /d "%OUT_DIR%"

if not exist "%EXE%" (
	echo ERROR: Executable not found: %EXE%
	pause >nul
	exit /b 1
)

set "CAN_LOG_DIR=CAN_LOGS"
if not exist "%CAN_LOG_DIR%" mkdir "%CAN_LOG_DIR%"

echo Starting VXL CAN capture (all connected interfaces)...
echo Press Ctrl+C to stop capture.
echo Output folder: %CAN_LOG_DIR%
"%EXE%" --test-can --can-backend vxl --can-listen-all --can-max-channels 64 --can-app CANoe --can-iface-version 4 --can-log-format asc --can-output-dir "%CAN_LOG_DIR%"
echo.
echo Capture stopped. Output saved under %CAN_LOG_DIR%.
echo Press any key to close.
pause >nul
