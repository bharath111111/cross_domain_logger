@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%.."

if exist "%SCRIPT_DIR%cross_domain_logger_windows.exe" (
	set "EXE=%SCRIPT_DIR%cross_domain_logger_windows.exe"
	set "DLL=%SCRIPT_DIR%vxlapi64.dll"
	set "OUT_DIR=%SCRIPT_DIR%"
) else (
	set "EXE=%ROOT_DIR%\target\release\cross_domain_logger.exe"
	set "DLL=%ROOT_DIR%\vxlapi64.dll"
	set "OUT_DIR=%ROOT_DIR%\"
)

cd /d "%OUT_DIR%"

if not exist "%EXE%" (
	echo ERROR: Executable not found: %EXE%
	pause >nul
	exit /b 1
)

if not exist "%DLL%" (
	echo ERROR: DLL not found: %DLL%
	pause >nul
	exit /b 1
)

set "CAN_LOG_DIR=CAN_LOGS"
if not exist "%CAN_LOG_DIR%" mkdir "%CAN_LOG_DIR%"

set "LOG_FILE=%CAN_LOG_DIR%\channel9.asc"
echo Starting continuous CAN capture on channel 9...
echo Press Ctrl+C to stop capture.
echo Output ASC file: %LOG_FILE%
"%EXE%" --test-can --can-listen --can-channel 9 --can-app CANoe --can-iface-version 4 --can-log-format asc --can-log-file "%LOG_FILE%"
echo.
echo Capture stopped. Output saved to %LOG_FILE%.
echo Press any key to close.
pause >nul
