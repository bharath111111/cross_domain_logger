@echo off
setlocal EnableExtensions
set "SCRIPT_DIR=%~dp0"
pushd "%SCRIPT_DIR%.." || exit /b 1

set "DIST_DIR=dist"
set "BUNDLE_DIR=%DIST_DIR%\_staging_bundle"
set "BUNDLE_ZIP=%DIST_DIR%\cross_domain_logger_windows_can_test_bundle.zip"
set "SHAREABLE_DIR=%DIST_DIR%\_staging_shareable"
set "SUMMARY_TXT=%SHAREABLE_DIR%\BUILD_CONFIG_SUMMARY.txt"
set "MASTER_ZIP=%DIST_DIR%\cross_domain_logger_shareable_package.zip"

echo [1/6] Building release binary...
cargo +stable-x86_64-pc-windows-gnu build --release --features vxl-can
if errorlevel 1 (
  echo ERROR: Build failed.
  exit /b 1
)

if not exist "target\release\cross_domain_logger.exe" (
  echo ERROR: Build output missing: target\release\cross_domain_logger.exe
  exit /b 1
)

echo [2/6] Preparing can-test bundle folder...
if exist "%BUNDLE_DIR%" rmdir /s /q "%BUNDLE_DIR%"
mkdir "%BUNDLE_DIR%"

echo [3/6] Copying runtime files into can-test bundle...
copy /y "target\release\cross_domain_logger.exe" "%BUNDLE_DIR%\cross_domain_logger_windows.exe" >nul
if errorlevel 1 (
  echo ERROR: Failed to copy executable into can-test bundle.
  popd
  exit /b 1
)
copy /y "vxlapi64.dll" "%BUNDLE_DIR%\vxlapi64.dll" >nul
if errorlevel 1 (
  echo ERROR: Failed to copy vxlapi64.dll into can-test bundle.
  popd
  exit /b 1
)
copy /y "scripts\run_can_test.bat" "%BUNDLE_DIR%\run_can_test.bat" >nul
if errorlevel 1 (
  echo ERROR: Failed to copy run_can_test.bat into can-test bundle.
  popd
  exit /b 1
)
copy /y "scripts\run_capture_ok_channels_parallel.bat" "%BUNDLE_DIR%\run_capture_ok_channels_parallel.bat" >nul
if errorlevel 1 (
  echo ERROR: Failed to copy run_capture_ok_channels_parallel.bat into can-test bundle.
  popd
  exit /b 1
)
copy /y "scripts\run_capture_all_connected_auto.bat" "%BUNDLE_DIR%\run_capture_all_connected_auto.bat" >nul
if errorlevel 1 (
  echo ERROR: Failed to copy run_capture_all_connected_auto.bat into can-test bundle.
  popd
  exit /b 1
)

if not exist "%BUNDLE_DIR%\cross_domain_logger_windows.exe" (
  echo ERROR: Can-test bundle exe is missing after copy.
  popd
  exit /b 1
)

echo [4/6] Creating can-test bundle ZIP...
if exist "%BUNDLE_ZIP%" del /f /q "%BUNDLE_ZIP%"
powershell -NoProfile -Command "$ErrorActionPreference='Stop'; Compress-Archive -Path '%BUNDLE_DIR%\*' -DestinationPath '%BUNDLE_ZIP%' -CompressionLevel Optimal"
if errorlevel 1 (
  echo ERROR: Failed to create cross_domain_logger_windows_can_test_bundle.zip
  popd
  exit /b 1
)

if not exist "%BUNDLE_ZIP%" (
  echo ERROR: cross_domain_logger_windows_can_test_bundle.zip not created.
  popd
  exit /b 1
)

echo [5/6] Preparing temporary shareable staging folder...
if exist "%SHAREABLE_DIR%" rmdir /s /q "%SHAREABLE_DIR%"
if not exist "%SHAREABLE_DIR%" mkdir "%SHAREABLE_DIR%"
copy /y "%BUNDLE_ZIP%" "%SHAREABLE_DIR%\cross_domain_logger_windows_can_test_bundle.zip" >nul
if errorlevel 1 (
  echo ERROR: Failed to stage cross_domain_logger_windows_can_test_bundle.zip for shareable package.
  popd
  exit /b 1
)
(
  echo Cross Domain Logger - Build Configuration Summary
  echo Generated: %date% %time%
  echo.
  echo Build command:
  echo   cargo +stable-x86_64-pc-windows-gnu build --release --features vxl-can
  echo.
  echo CAN runtime defaults:
  echo   App name: CANoe
  echo   Interface version: 4
  echo   User display channels: 1 to 11
  echo   Internal app channels: 0 to 10
  echo.
  echo Channel mapping ^(user channel to app channel to VN to Network^):
  echo   1  to 0  to vn 1670 1 to FD_CANW
  echo   2  to 1  to vn 1670 1 to FD_CAN5
  echo   3  to 2  to vn 1670 2 to FD_CAN9
  echo   4  to 3  to vn 1670 2 to FD_CAN13
  echo   5  to 4  to vn 1670 2 to FD_CAN14
  echo   6  to 5  to vn 1670 1 to FD_CAN15
  echo   7  to 6  to vn 1670 1 to FD_CAN17
  echo   8  to 7  to vn 1670 1 to FD_CAN18
  echo   9  to 8  to vn 1670 1 to FD_CAN20
  echo   10 to 9  to vn 1670 1 to FD_CAN21
  echo   11 to 10 to vn 1670 1 to HS_CAN1
  echo.
  echo Output naming:
  echo   ASC files are written using network names ^(for example FD_CANW.asc^).
) > "%SUMMARY_TXT%"

if not exist "%SUMMARY_TXT%" (
  echo ERROR: Failed to generate BUILD_CONFIG_SUMMARY.txt
  popd
  exit /b 1
)

echo [6/6] Creating master shareable package ZIP...
if exist "%MASTER_ZIP%" del /f /q "%MASTER_ZIP%"
set "ZIP_OK=0"
for /l %%I in (1,1,3) do (
  powershell -NoProfile -Command "$ErrorActionPreference='Stop'; Compress-Archive -Path '%SHAREABLE_DIR%\*' -DestinationPath '%MASTER_ZIP%' -CompressionLevel Optimal"
  if not errorlevel 1 (
    set "ZIP_OK=1"
    goto :zip_done
  )
  if %%I lss 3 (
    echo WARN: Master zip attempt %%I failed, retrying...
    timeout /t 2 /nobreak >nul
  )
)

:zip_done
if not "%ZIP_OK%"=="1" (
  echo ERROR: Failed to create cross_domain_logger_shareable_package.zip
  popd
  exit /b 1
)

if not exist "%MASTER_ZIP%" (
  echo ERROR: cross_domain_logger_shareable_package.zip not created.
  popd
  exit /b 1
)

if exist "%BUNDLE_DIR%" rmdir /s /q "%BUNDLE_DIR%"
if exist "%SHAREABLE_DIR%" rmdir /s /q "%SHAREABLE_DIR%"

echo.
echo DONE:
echo  - %BUNDLE_ZIP%
echo  - %MASTER_ZIP%
popd
exit /b 0
