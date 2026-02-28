@echo off
setlocal
cd /d "%~dp0"
echo Running CAN test...
cross_domain_logger_windows.exe --test-can
echo.
echo Output saved only in console. Press any key to close.
pause >nul
