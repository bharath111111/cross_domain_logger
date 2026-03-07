@echo off
setlocal
call "%~dp0scripts\run_eth_stlb_capture.bat" %*
exit /b %ERRORLEVEL%
