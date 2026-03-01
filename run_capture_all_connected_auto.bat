@echo off
setlocal
call "%~dp0scripts\run_capture_all_connected_auto.bat" %*
exit /b %ERRORLEVEL%
