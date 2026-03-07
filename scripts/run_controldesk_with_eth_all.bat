@echo off
setlocal EnableExtensions EnableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%.."

if exist "%SCRIPT_DIR%cross_domain_logger_windows.exe" (
  set "EXE=%SCRIPT_DIR%cross_domain_logger_windows.exe"
  set "WORK_DIR=%SCRIPT_DIR%"
) else (
  set "EXE=%ROOT_DIR%\target\release\cross_domain_logger.exe"
  set "WORK_DIR=%ROOT_DIR%\"
)

cd /d "%WORK_DIR%" || exit /b 1

set "OUT_DIR=CAN_LOGS"
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"

set "RUN_LOG=%OUT_DIR%\run_controldesk_with_eth_all.log"
echo ==== START %date% %time% ==== > "%RUN_LOG%"
echo Working dir: %cd% >> "%RUN_LOG%"

set "CAPTURE_MS=%~1"
if not defined CAPTURE_MS set "CAPTURE_MS=60000"
set /a CAPTURE_SEC=(%CAPTURE_MS%+999)/1000

if not exist "%EXE%" (
  echo ERROR: Executable not found: %EXE%
  echo ERROR: Executable not found >> "%RUN_LOG%"
  goto :end_error
)

set "PCAP_CMD="
where dumpcap >nul 2>&1
if not errorlevel 1 set "PCAP_CMD=dumpcap"

if not defined PCAP_CMD (
  where tshark >nul 2>&1
  if not errorlevel 1 set "PCAP_CMD=tshark"
)

if not defined PCAP_CMD if exist "C:\Program Files\Wireshark\dumpcap.exe" set "PCAP_CMD=C:\Program Files\Wireshark\dumpcap.exe"
if not defined PCAP_CMD if exist "C:\Program Files\Wireshark\tshark.exe" set "PCAP_CMD=C:\Program Files\Wireshark\tshark.exe"
if not defined PCAP_CMD if exist "C:\Program Files (x86)\Wireshark\dumpcap.exe" set "PCAP_CMD=C:\Program Files (x86)\Wireshark\dumpcap.exe"
if not defined PCAP_CMD if exist "C:\Program Files (x86)\Wireshark\tshark.exe" set "PCAP_CMD=C:\Program Files (x86)\Wireshark\tshark.exe"

if not defined PCAP_CMD (
  echo ERROR: Neither dumpcap nor tshark was found.
  echo Install Wireshark ^(+Npcap^) or add Wireshark folder to PATH.
  echo ERROR: capture tool missing >> "%RUN_LOG%"
  goto :end_error
)

set "ETH_IF_ARGS="
for /f "tokens=1,* delims=." %%A in ('"%PCAP_CMD%" -D 2^>nul') do (
  set "IF_NUM=%%A"
  set "IF_LINE=%%B"
  if defined IF_NUM (
    echo !IF_LINE! | findstr /i "ethernet" >nul
    if not errorlevel 1 (
      set "ETH_IF_ARGS=!ETH_IF_ARGS! -i !IF_NUM!"
    )
  )
)

if not defined ETH_IF_ARGS (
  echo WARN: No Ethernet interfaces matched from dumpcap -D output.
  echo WARN: No Ethernet interfaces matched. >> "%RUN_LOG%"
) else (
  echo Starting Ethernet capture on all detected Ethernet interfaces...
  echo capture tool: %PCAP_CMD%
  echo capture tool: %PCAP_CMD% >> "%RUN_LOG%"
  echo interface args: !ETH_IF_ARGS!
  echo interface args: !ETH_IF_ARGS! >> "%RUN_LOG%"
  start "ETH_ALL_CAPTURE" /min cmd /c "\"%PCAP_CMD%\" !ETH_IF_ARGS! -a duration:%CAPTURE_SEC% -w \"%OUT_DIR%\ethernet_all.pcapng\" 1> \"%OUT_DIR%\ethernet_console.log\" 2>&1"
)

echo Starting ControlDesk bus capture for %CAPTURE_MS% ms...
echo Output folder: %OUT_DIR%
echo Running ControlDesk capture... >> "%RUN_LOG%"

"%EXE%" --test-can --can-backend controldesk --can-listen-all --can-duration-ms %CAPTURE_MS% --can-log-format asc --can-output-dir "%OUT_DIR%" >> "%RUN_LOG%" 2>&1
set "CAN_ERR=%ERRORLEVEL%"

if not "%CAN_ERR%"=="0" (
  echo ERROR: ControlDesk capture failed with exit code %CAN_ERR%.
  echo ERROR: ControlDesk capture failed with exit code %CAN_ERR%. >> "%RUN_LOG%"
  goto :end_error
)

echo Capture completed.
if exist "%OUT_DIR%\ethernet_all.pcapng" echo Ethernet file: %OUT_DIR%\ethernet_all.pcapng
echo ControlDesk logs: %OUT_DIR%
echo ==== END OK %date% %time% ==== >> "%RUN_LOG%"

if defined NO_PAUSE exit /b 0
echo Press any key to close.
pause >nul
exit /b 0

:end_error
echo ==== END ERROR %date% %time% ==== >> "%RUN_LOG%"
if defined NO_PAUSE exit /b 1
echo Press any key to close.
pause >nul
exit /b 1
