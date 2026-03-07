@echo off
setlocal
call "%~dp0scripts\run_make_triage_pack.bat" %*
exit /b %ERRORLEVEL%
