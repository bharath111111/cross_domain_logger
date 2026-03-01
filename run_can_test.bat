@echo off
setlocal
call "%~dp0scripts\run_can_test.bat" %*
exit /b %ERRORLEVEL%
