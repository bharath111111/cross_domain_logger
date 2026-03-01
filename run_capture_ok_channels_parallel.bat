@echo off
setlocal
call "%~dp0scripts\run_capture_ok_channels_parallel.bat" %*
exit /b %ERRORLEVEL%
