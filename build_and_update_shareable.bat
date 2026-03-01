@echo off
setlocal
call "%~dp0scripts\build_and_update_shareable.bat" %*
exit /b %ERRORLEVEL%
