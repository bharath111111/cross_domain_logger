@echo off
setlocal
cd /d "%~dp0"
set "LOG_FILE=channel9.asc"
echo Starting continuous CAN capture on channel 9...
echo Press Ctrl+C to stop capture.
echo Output ASC file: %LOG_FILE%
cross_domain_logger_windows.exe --test-can --can-listen --can-channel 9 --can-app CANoe --can-iface-version 4 --can-log-format asc --can-log-file "%LOG_FILE%"
echo.
echo Capture stopped. Output saved to %LOG_FILE%.
echo Press any key to close.
pause >nul
