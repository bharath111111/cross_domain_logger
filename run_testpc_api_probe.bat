@echo off
setlocal
call "%~dp0scripts\run_testpc_api_probe.bat" %*
exit /b %ERRORLEVEL%
