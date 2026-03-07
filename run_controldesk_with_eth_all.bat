@echo off
setlocal
call "%~dp0scripts\run_controldesk_with_eth_all.bat" %*
exit /b %ERRORLEVEL%
