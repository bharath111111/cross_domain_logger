@echo off
setlocal EnableExtensions EnableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
set "ROOT_DIR=%SCRIPT_DIR%.."

if exist "%SCRIPT_DIR%cross_domain_logger_windows.exe" (
  set "WORK_DIR=%SCRIPT_DIR%"
) else (
  set "WORK_DIR=%ROOT_DIR%\"
)

cd /d "%WORK_DIR%" || exit /b 1

set "OUT_DIR=CAN_LOGS"
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"

set "CAPTURE_MS=%~1"
if not defined CAPTURE_MS set "CAPTURE_MS=60000"
set /a CAPTURE_SEC=(%CAPTURE_MS%+999)/1000

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
  if defined NO_PAUSE exit /b 1
  pause
  exit /b 1
)

set "STLB_ARGS="
for /f "tokens=1,* delims=." %%A in ('"%PCAP_CMD%" -D 2^>nul') do (
  set "IF_NUM=%%A"
  set "IF_LINE=%%B"
  if defined IF_NUM (
    echo !IF_LINE! | findstr /i "stlb" >nul
    if not errorlevel 1 (
      set "STLB_ARGS=!STLB_ARGS! -i !IF_NUM!"
    )
  )
)

if not defined STLB_ARGS (
  echo WARN: No interface name containing STLB found.
  echo Falling back to all Ethernet interfaces.
  for /f "tokens=1,* delims=." %%A in ('"%PCAP_CMD%" -D 2^>nul') do (
    set "IF_NUM=%%A"
    set "IF_LINE=%%B"
    if defined IF_NUM (
      echo !IF_LINE! | findstr /i "ethernet" >nul
      if not errorlevel 1 (
        set "STLB_ARGS=!STLB_ARGS! -i !IF_NUM!"
      )
    )
  )
)

if not defined STLB_ARGS (
  echo ERROR: Could not resolve any Ethernet interfaces for capture.
  if defined NO_PAUSE exit /b 1
  pause
  exit /b 1
)

set "OUT_FILE=%OUT_DIR%\ETH_STLB.pcapng"
echo Capturing ETH_STLB traffic for %CAPTURE_SEC% sec...
echo Capture tool: %PCAP_CMD%
echo Interfaces:%STLB_ARGS%
echo Output: %OUT_FILE%

"%PCAP_CMD%" %STLB_ARGS% -a duration:%CAPTURE_SEC% -w "%OUT_FILE%"
set "ERR=%ERRORLEVEL%"

if not "%ERR%"=="0" (
  echo ERROR: dumpcap failed with exit code %ERR%.
  if defined NO_PAUSE exit /b %ERR%
  pause
  exit /b %ERR%
)

echo Capture complete: %OUT_FILE%
if defined NO_PAUSE exit /b 0
pause
exit /b 0
