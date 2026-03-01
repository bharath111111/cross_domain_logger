// CAN/ETH capture via vxlapi.dll (Vector hardware)
// This is a standalone component, not interfering with the main logger.
// Only loaded/used if enabled in the main app.

// --- Begin inlined vxlapi.dll FFI bindings ---
use std::os::raw::{c_char, c_short, c_uint, c_ulonglong, c_void};

#[allow(non_camel_case_types)]
pub type XLstatus = c_short;
#[allow(non_camel_case_types)]
pub type XLportHandle = c_uint;
#[allow(non_camel_case_types)]
pub type XLaccess = c_ulonglong;
#[allow(non_camel_case_types)]
pub type XLdriverConfig = c_void;

const XL_SUCCESS: XLstatus = 0;
const XL_ERR_QUEUE_IS_EMPTY: XLstatus = 10;
const XL_CAN_EV_TAG_RX_OK: u16 = 0x0400;
const XL_CAN_EV_TAG_TX_OK: u16 = 0x0404;
const XL_OUTPUT_MODE_NORMAL: i32 = 1;

#[repr(C)]
pub struct XLcanFdConf {
    pub arbitrationBitRate: u32,
    pub sjwAbr: u32,
    pub tseg1Abr: u32,
    pub tseg2Abr: u32,
    pub dataBitRate: u32,
    pub sjwDbr: u32,
    pub tseg1Dbr: u32,
    pub tseg2Dbr: u32,
    pub reserved: u8,
    pub options: u8,
    pub reserved1: [u8; 2],
    pub reserved2: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XLCanRxMsg {
    pub canId: u32,
    pub msgFlags: u32,
    pub crc: u32,
    pub reserved1: [u8; 12],
    pub totalBitCnt: u16,
    pub dlc: u8,
    pub reserved: [u8; 5],
    pub data: [u8; 64],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union XLcanRxTagData {
    pub raw: [u8; 96],
    pub canRxOkMsg: XLCanRxMsg,
    pub canTxOkMsg: XLCanRxMsg,
}

#[repr(C)]
pub struct XLcanRxEvent {
    pub size: u32,
    pub tag: u16,
    pub channelIndex: u16,
    pub userHandle: u32,
    pub flagsChip: u16,
    pub reserved0: u16,
    pub reserved1: u64,
    pub timeStampSync: u64,
    pub tagData: XLcanRxTagData,
}

#[cfg_attr(all(target_os = "windows", target_arch = "x86_64"), link(name = "vxlapi64"))]
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86_64")), link(name = "vxlapi"))]
unsafe extern "C" {
    pub fn xlOpenDriver() -> XLstatus;
    pub fn xlCloseDriver() -> XLstatus;
    pub fn xlGetDriverConfig(pDriverConfig: *mut XLdriverConfig) -> XLstatus;
    pub fn xlGetApplConfig(
        appName: *mut c_char,
        appChannel: u32,
        pHwType: *mut u32,
        pHwIndex: *mut u32,
        pHwChannel: *mut u32,
        busType: u32,
    ) -> XLstatus;
    pub fn xlGetChannelMask(hwType: i32, hwIndex: i32, hwChannel: i32) -> XLaccess;
    pub fn xlOpenPort(portHandle: *mut XLportHandle, appName: *const c_char, accessMask: XLaccess, permissionMask: *mut XLaccess, rxQueueSize: u32, xlInterfaceVersion: u32, busType: u32) -> XLstatus;
    pub fn xlSetNotification(portHandle: XLportHandle, hwnd: *mut c_void, msg: u32) -> XLstatus;
    pub fn xlActivateChannel(portHandle: XLportHandle, accessMask: XLaccess, busType: u32, flags: u32) -> XLstatus;
    pub fn xlCanReceive(portHandle: XLportHandle, pXlCanRxEvt: *mut XLcanRxEvent) -> XLstatus;
    pub fn xlCanGetEventString(pEv: *mut XLcanRxEvent) -> *const c_char;
    pub fn xlGetErrorString(err: XLstatus) -> *const c_char;
    pub fn xlCanSetChannelOutput(portHandle: XLportHandle, accessMask: XLaccess, mode: i32) -> XLstatus;
    pub fn xlCanFdSetConfiguration(portHandle: XLportHandle, accessMask: XLaccess, pCanFdConf: *mut XLcanFdConf) -> XLstatus;
    pub fn xlDeactivateChannel(portHandle: XLportHandle, accessMask: XLaccess) -> XLstatus;
    pub fn xlClosePort(portHandle: XLportHandle) -> XLstatus;
}
// --- End inlined vxlapi.dll FFI bindings ---
use std::ffi::{CStr, CString};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Copy, Clone)]
pub enum CanLogFormat {
    Text,
    Asc,
}

pub fn try_open_driver() -> Result<(), String> {
    let status = unsafe { xlOpenDriver() };
    if status == XL_SUCCESS {
        Ok(())
    } else {
        Err(format!("xlOpenDriver failed: status {} ({})", status, xl_error_string(status)))
    }
}

pub fn try_close_driver() {
    unsafe { xlCloseDriver() };
}

// Add more wrappers for channel/network selection, CAN/ETH capture, etc.

// Example usage (to be called from main app):
// vxl_capture::try_open_driver();
// ...
// vxl_capture::try_close_driver();
pub fn try_capture_can() -> Result<(), String> {
    match try_capture_can_on_channel(0, 1000, "RustApp", 4)? {
        true => Ok(()),
        false => Err("No CAN event observed on channel 0 within timeout".to_string()),
    }
}

pub fn try_capture_can_on_channel(
    app_channel: u32,
    timeout_ms: u64,
    app_name: &str,
    interface_version: u32,
) -> Result<bool, String> {
    const XL_BUS_TYPE_CAN: u32 = 1;

    if app_channel >= 64 {
        return Err(format!("Invalid app channel {} (must be < 64)", app_channel));
    }

    let app_name = CString::new(app_name)
        .map_err(|_| "Invalid app name (contains NUL byte)".to_string())?;

    let mut hw_type = 0u32;
    let mut hw_index = 0u32;
    let mut hw_channel = 0u32;
    let app_cfg_status = unsafe {
        xlGetApplConfig(
            app_name.as_ptr() as *mut c_char,
            app_channel,
            &mut hw_type as *mut _,
            &mut hw_index as *mut _,
            &mut hw_channel as *mut _,
            XL_BUS_TYPE_CAN,
        )
    };
    let access_mask = if app_cfg_status == XL_SUCCESS {
        let mapped_mask = unsafe { xlGetChannelMask(hw_type as i32, hw_index as i32, hw_channel as i32) };
        if mapped_mask == 0 {
            return Err(format!(
                "No channel mask for app '{}' channel {} (hwType={}, hwIndex={}, hwChannel={})",
                app_name.to_string_lossy(),
                app_channel,
                hw_type,
                hw_index,
                hw_channel
            ));
        }
        mapped_mask
    } else {
        let all_mask = unsafe { xlGetChannelMask(-1, -1, -1) };
        if all_mask == 0 {
            return Err(format!(
                "xlGetApplConfig failed for app '{}' channel {}: status {} ({}), and no wildcard channel mask available",
                app_name.to_string_lossy(),
                app_channel,
                app_cfg_status,
                xl_error_string(app_cfg_status)
            ));
        }

        match nth_set_bit_mask(all_mask, app_channel as usize) {
            Some(mask) => {
                println!(
                    "Using fallback hardware channel ordinal {} (display ch {}, app mapping missing for '{}')",
                    app_channel,
                    app_channel + 1,
                    app_name.to_string_lossy()
                );
                mask
            }
            None => {
                return Err(format!(
                    "App mapping missing for '{}' and fallback ordinal {} exceeds available channels",
                    app_name.to_string_lossy(),
                    app_channel
                ));
            }
        }
    };

    let mut port_handle: XLportHandle = 0;
    let mut permission_mask: XLaccess = access_mask;
    let rx_queue_size = 16384u32;
    let bus_type = XL_BUS_TYPE_CAN;

    // Open port
    let status = unsafe {
        xlOpenPort(
            &mut port_handle as *mut _,
            app_name.as_ptr(),
            access_mask,
            &mut permission_mask as *mut _,
            rx_queue_size,
            interface_version,
            bus_type,
        )
    };
    if status != XL_SUCCESS {
        return Err(format!("xlOpenPort failed: status {} ({})", status, xl_error_string(status)));
    }

    if let Err(e) = apply_canfd_defaults(port_handle, access_mask) {
        unsafe { xlClosePort(port_handle) };
        return Err(e);
    }

    // Activate channel
    let status = unsafe { xlActivateChannel(port_handle, access_mask, bus_type, 0) };
    if status != XL_SUCCESS {
        unsafe { xlClosePort(port_handle) };
        return Err(format!("xlActivateChannel failed: status {} ({})", status, xl_error_string(status)));
    }

    let timeout = Duration::from_millis(timeout_ms);
    let start = Instant::now();
    let mut received = false;

    while start.elapsed() < timeout {
        let mut event = XLcanRxEvent {
            size: 0,
            tag: 0,
            channelIndex: 0,
            userHandle: 0,
            flagsChip: 0,
            reserved0: 0,
            reserved1: 0,
            timeStampSync: 0,
            tagData: XLcanRxTagData { raw: [0; 96] },
        };

        let status = unsafe { xlCanReceive(port_handle, &mut event as *mut _) };

        if status == XL_SUCCESS {
            if event.tag == XL_CAN_EV_TAG_RX_OK || event.tag == XL_CAN_EV_TAG_TX_OK {
                let msg = unsafe { event.tagData.canRxOkMsg };
                let data_len = can_dlc_to_len(msg.dlc).min(msg.data.len());
                println!(
                    "Received CAN channel={} (appCh={}) id=0x{:X} dlc={} data={:02X?}",
                    event.channelIndex as u32 + 1,
                    app_channel + 1,
                    msg.canId,
                    msg.dlc,
                    &msg.data[..data_len]
                );
                received = true;
                break;
            }

            let event_text = unsafe {
                let ptr = xlCanGetEventString(&mut event as *mut _);
                if ptr.is_null() {
                    String::from("unknown event")
                } else {
                    CStr::from_ptr(ptr).to_string_lossy().into_owned()
                }
            };
            println!(
                "CAN event tag=0x{:X} channel={} ({})",
                event.tag,
                event.channelIndex as u32 + 1,
                event_text
            );
            continue;
        }

        if status == XL_ERR_QUEUE_IS_EMPTY {
            thread::sleep(Duration::from_millis(10));
            continue;
        }

        unsafe {
            xlDeactivateChannel(port_handle, access_mask);
            xlClosePort(port_handle);
        }
        return Err(format!(
            "xlCanReceive failed: status {} ({})",
            status,
            xl_error_string(status)
        ));
    }

    unsafe {
        xlDeactivateChannel(port_handle, access_mask);
        xlClosePort(port_handle);
    }

    Ok(received)
}

fn can_dlc_to_len(dlc: u8) -> usize {
    match dlc {
        0..=8 => dlc as usize,
        9 => 12,
        10 => 16,
        11 => 20,
        12 => 24,
        13 => 32,
        14 => 48,
        _ => 64,
    }
}

fn xl_error_string(status: XLstatus) -> String {
    unsafe {
        let ptr = xlGetErrorString(status);
        if ptr.is_null() {
            return "unknown error".to_string();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

fn apply_canfd_defaults(port_handle: XLportHandle, access_mask: XLaccess) -> Result<(), String> {
    let output_status = unsafe { xlCanSetChannelOutput(port_handle, access_mask, XL_OUTPUT_MODE_NORMAL) };
    if output_status != XL_SUCCESS {
        return Err(format!(
            "xlCanSetChannelOutput failed: status {} ({})",
            output_status,
            xl_error_string(output_status)
        ));
    }

    let mut fd_conf = XLcanFdConf {
        arbitrationBitRate: 500_000,
        sjwAbr: 16,
        tseg1Abr: 63,
        tseg2Abr: 16,
        dataBitRate: 2_000_000,
        sjwDbr: 10,
        tseg1Dbr: 29,
        tseg2Dbr: 10,
        reserved: 0,
        options: 0,
        reserved1: [0; 2],
        reserved2: 0,
    };

    let fd_status = unsafe { xlCanFdSetConfiguration(port_handle, access_mask, &mut fd_conf as *mut _) };
    if fd_status != XL_SUCCESS {
        return Err(format!(
            "xlCanFdSetConfiguration failed: status {} ({})",
            fd_status,
            xl_error_string(fd_status)
        ));
    }

    Ok(())
}

fn nth_set_bit_mask(mask: XLaccess, ordinal: usize) -> Option<XLaccess> {
    let mut found = 0usize;
    for bit in 0..64 {
        let bit_mask = 1u64 << bit;
        if (mask & bit_mask) != 0 {
            if found == ordinal {
                return Some(bit_mask);
            }
            found += 1;
        }
    }
    None
}

pub fn try_capture_any_can(
    max_channels: u32,
    timeout_per_channel_ms: u64,
    app_name: &str,
    interface_version: u32,
) -> Result<u32, String> {
    let mut denied_111_count = 0u32;
    let mut open_port_error_count = 0u32;

    for channel in 0..max_channels {
        println!("Trying CAN channel {}...", channel + 1);
        match try_capture_can_on_channel(channel, timeout_per_channel_ms, app_name, interface_version) {
            Ok(true) => return Ok(channel),
            Ok(false) => {}
            Err(e) => {
                if e.contains("status 111") {
                    denied_111_count += 1;
                }
                if e.contains("xlOpenPort failed") {
                    open_port_error_count += 1;
                }
                println!("Channel {} not usable: {}", channel + 1, e);
            }
        }
    }

    if denied_111_count == max_channels && max_channels > 0 {
        return Err(format!(
            "All {} channels denied by XL Driver (xlOpenPort status 111). Configure XL channel access for app '{}' and retry.",
            max_channels,
            app_name
        ));
    }

    if open_port_error_count == max_channels && max_channels > 0 {
        return Err(format!(
            "Unable to open any channel ({} open-port failures). Verify app name '{}', interface version {}, and channel mapping.",
            open_port_error_count,
            app_name,
            interface_version
        ));
    }

    Err(format!(
        "No CAN traffic detected on channels 0..{}",
        max_channels.saturating_sub(1)
    ))
}

pub fn diagnose_can_setup(max_channels: u32, app_name: &str, interface_version: u32) {
    println!(
        "CAN Diagnose: app='{}', iface={}, scan-channels=0..{}",
        app_name,
        interface_version,
        max_channels.saturating_sub(1)
    );

    let mut channels_with_frames = 0u32;
    let mut channels_open_no_frames = 0u32;
    let mut mapping_errors = 0u32;
    let mut invalid_channel_errors = 0u32;
    let mut open_port_errors = 0u32;

    for channel in 0..max_channels {
        match try_capture_can_on_channel(channel, 250, app_name, interface_version) {
            Ok(true) => {
                channels_with_frames += 1;
                println!("DIAG channel {}: OK (frames detected)", channel + 1);
            }
            Ok(false) => {
                channels_open_no_frames += 1;
                println!("DIAG channel {}: OK (no frames during 250ms)", channel + 1);
            }
            Err(e) => {
                if e.contains("xlGetApplConfig failed") {
                    mapping_errors += 1;
                }
                if e.contains("XL_ERR_INVALID_CHAN_INDEX") {
                    invalid_channel_errors += 1;
                }
                if e.contains("xlOpenPort failed") {
                    open_port_errors += 1;
                }
                println!("DIAG channel {}: {}", channel + 1, e);
            }
        }
    }

    println!(
        "DIAG summary: frames={}, open_no_frames={}, mapping_errors={}, invalid_channel_errors={}, open_port_errors={}",
        channels_with_frames,
        channels_open_no_frames,
        mapping_errors,
        invalid_channel_errors,
        open_port_errors
    );

    if channels_with_frames > 0 {
        println!("DIAG result: CAN capture path is working.");
        return;
    }

    if channels_open_no_frames > 0 {
        println!("DIAG result: channels open successfully, but no traffic was observed in test window.");
        return;
    }

    if mapping_errors == max_channels {
        println!(
            "DIAG hint: application mapping for '{}' is missing. Configure at least one app channel in Vector XL Driver Configuration.",
            app_name
        );
    }

    if invalid_channel_errors == max_channels {
        println!(
            "DIAG hint: channel ordinals are not valid for current setup. Re-check app-channel mapping indices.");
    }

    if open_port_errors == max_channels {
        println!(
            "DIAG hint: all channel opens failed. Verify XL permissions, app name, and interface version.");
    }
}

pub fn print_can_channel_mapping(max_channels: u32, app_name: &str) {
    const XL_BUS_TYPE_CAN: u32 = 1;
    let app_name_c = match CString::new(app_name) {
        Ok(s) => s,
        Err(_) => {
            println!("Invalid app name '{}': contains NUL byte", app_name);
            return;
        }
    };

    println!(
        "CAN mapping for app '{}': channel(1-based) -> hwType/hwIndex/hwChannel (mask) -> VN -> Network",
        app_name
    );

    let mut found = 0u32;
    for app_channel in 0..max_channels {
        let mut hw_type = 0u32;
        let mut hw_index = 0u32;
        let mut hw_channel = 0u32;

        let status = unsafe {
            xlGetApplConfig(
                app_name_c.as_ptr() as *mut c_char,
                app_channel,
                &mut hw_type as *mut _,
                &mut hw_index as *mut _,
                &mut hw_channel as *mut _,
                XL_BUS_TYPE_CAN,
            )
        };

        if status == XL_SUCCESS {
            let mask = unsafe { xlGetChannelMask(hw_type as i32, hw_index as i32, hw_channel as i32) };
            let display_channel = app_channel + 1;
            let (vn_label, network_name) = can_network_alias(app_channel)
                .unwrap_or(("UNMAPPED", "UNMAPPED"));
            println!(
                "ch={} (appCh={}) -> hwType={} hwIndex={} hwChannel={} mask=0x{:X} -> {} -> {}",
                display_channel,
                app_channel,
                hw_type,
                hw_index,
                hw_channel,
                mask,
                vn_label,
                network_name
            );
            found += 1;
        }
    }

    if found == 0 {
        println!("No CAN app-channel mappings found for app '{}'.", app_name);
    }
}

fn can_network_alias(app_channel: u32) -> Option<(&'static str, &'static str)> {
    match app_channel {
        0 => Some(("vn 1670 1", "FD_CANW")),
        1 => Some(("vn 1670 1", "FD_CAN5")),
        2 => Some(("vn 1670 2", "FD_CAN9")),
        3 => Some(("vn 1670 2", "FD_CAN13")),
        4 => Some(("vn 1670 2", "FD_CAN14")),
        5 => Some(("vn 1670 1", "FD_CAN15")),
        6 => Some(("vn 1670 1", "FD_CAN17")),
        7 => Some(("vn 1670 1", "FD_CAN18")),
        8 => Some(("vn 1670 1", "FD_CAN20")),
        9 => Some(("vn 1670 1", "FD_CAN21")),
        10 => Some(("vn 1670 1", "HS_CAN1")),
        _ => None,
    }
}

fn can_output_file_stem(app_channel: u32) -> String {
    if let Some((_, network_name)) = can_network_alias(app_channel) {
        network_name.to_string()
    } else {
        format!("channel{}", app_channel + 1)
    }
}

pub fn listen_can_on_channel(
    app_channel: u32,
    app_name: &str,
    interface_version: u32,
    duration_ms: Option<u64>,
    log_file_path: Option<&str>,
    log_format: CanLogFormat,
) -> Result<(), String> {
    const XL_BUS_TYPE_CAN: u32 = 1;

    if app_channel >= 64 {
        return Err(format!("Invalid app channel {} (must be < 64)", app_channel));
    }

    let app_name = CString::new(app_name)
        .map_err(|_| "Invalid app name (contains NUL byte)".to_string())?;

    let mut hw_type = 0u32;
    let mut hw_index = 0u32;
    let mut hw_channel = 0u32;
    let app_cfg_status = unsafe {
        xlGetApplConfig(
            app_name.as_ptr() as *mut c_char,
            app_channel,
            &mut hw_type as *mut _,
            &mut hw_index as *mut _,
            &mut hw_channel as *mut _,
            XL_BUS_TYPE_CAN,
        )
    };

    let access_mask = if app_cfg_status == XL_SUCCESS {
        let mapped_mask = unsafe { xlGetChannelMask(hw_type as i32, hw_index as i32, hw_channel as i32) };
        if mapped_mask == 0 {
            return Err(format!(
                "No channel mask for app '{}' channel {} (hwType={}, hwIndex={}, hwChannel={})",
                app_name.to_string_lossy(),
                app_channel,
                hw_type,
                hw_index,
                hw_channel
            ));
        }
        mapped_mask
    } else {
        let all_mask = unsafe { xlGetChannelMask(-1, -1, -1) };
        if all_mask == 0 {
            return Err(format!(
                "xlGetApplConfig failed for app '{}' channel {}: status {} ({}), and no wildcard channel mask available",
                app_name.to_string_lossy(),
                app_channel,
                app_cfg_status,
                xl_error_string(app_cfg_status)
            ));
        }

        match nth_set_bit_mask(all_mask, app_channel as usize) {
            Some(mask) => {
                println!(
                    "Using fallback hardware channel ordinal {} (app mapping missing for '{}')",
                    app_channel,
                    app_name.to_string_lossy()
                );
                mask
            }
            None => {
                return Err(format!(
                    "App mapping missing for '{}' and fallback ordinal {} exceeds available channels",
                    app_name.to_string_lossy(),
                    app_channel
                ));
            }
        }
    };

    let mut port_handle: XLportHandle = 0;
    let mut permission_mask: XLaccess = access_mask;
    let rx_queue_size = 16384u32;

    let status = unsafe {
        xlOpenPort(
            &mut port_handle as *mut _,
            app_name.as_ptr(),
            access_mask,
            &mut permission_mask as *mut _,
            rx_queue_size,
            interface_version,
            XL_BUS_TYPE_CAN,
        )
    };
    if status != XL_SUCCESS {
        return Err(format!("xlOpenPort failed: status {} ({})", status, xl_error_string(status)));
    }

    if let Err(e) = apply_canfd_defaults(port_handle, access_mask) {
        unsafe { xlClosePort(port_handle) };
        return Err(e);
    }

    let status = unsafe { xlActivateChannel(port_handle, access_mask, XL_BUS_TYPE_CAN, 0) };
    if status != XL_SUCCESS {
        unsafe { xlClosePort(port_handle) };
        return Err(format!("xlActivateChannel failed: status {} ({})", status, xl_error_string(status)));
    }

    let mut log_file = if let Some(path) = log_file_path {
        Some(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(path)
                .map_err(|e| format!("Failed to open log file '{}': {}", path, e))?,
        )
    } else {
        None
    };

    if let Some(file) = &mut log_file {
        match log_format {
            CanLogFormat::Text => {}
            CanLogFormat::Asc => {
                let _ = writeln!(file, "date {}", chrono::Local::now().format("%a %b %d %H:%M:%S%.3f %Y"));
                let _ = writeln!(file, "base hex  timestamps relative");
                let _ = writeln!(file, "internal events logged");
                let _ = writeln!(file, "Begin Triggerblock {}", chrono::Local::now().format("%a %b %d %H:%M:%S%.3f %Y"));
            }
        }
    }

    println!(
        "Listening continuously on CAN app-channel {}. Press Ctrl+C to stop.",
        app_channel + 1
    );

    let start = Instant::now();
    let mut frame_count: u64 = 0;

    loop {
        if let Some(ms) = duration_ms {
            if start.elapsed() >= Duration::from_millis(ms) {
                break;
            }
        }

        let mut event = XLcanRxEvent {
            size: 0,
            tag: 0,
            channelIndex: 0,
            userHandle: 0,
            flagsChip: 0,
            reserved0: 0,
            reserved1: 0,
            timeStampSync: 0,
            tagData: XLcanRxTagData { raw: [0; 96] },
        };

        let status = unsafe { xlCanReceive(port_handle, &mut event as *mut _) };

        if status == XL_SUCCESS {
            if event.tag == XL_CAN_EV_TAG_RX_OK || event.tag == XL_CAN_EV_TAG_TX_OK {
                let msg = unsafe { event.tagData.canRxOkMsg };
                let data_len = can_dlc_to_len(msg.dlc).min(msg.data.len());
                frame_count += 1;
                let line = format!(
                    "frame={} channel={} id=0x{:X} dlc={} data={:02X?}",
                    frame_count,
                    event.channelIndex,
                    msg.canId,
                    msg.dlc,
                    &msg.data[..data_len]
                );
                println!("{}", line);
                if let Some(file) = &mut log_file {
                    match log_format {
                        CanLogFormat::Text => {
                            let _ = writeln!(file, "{}", line);
                        }
                        CanLogFormat::Asc => {
                            let ts = start.elapsed().as_secs_f64();
                            let channel = event.channelIndex as u32 + 1;
                            let id = msg.canId & 0x1FFF_FFFF;
                            let mut bytes = String::new();
                            for (i, b) in msg.data[..data_len].iter().enumerate() {
                                if i > 0 {
                                    bytes.push(' ');
                                }
                                bytes.push_str(&format!("{:02X}", b));
                            }
                            let _ = writeln!(
                                file,
                                "{:.6} {} {:X} Rx d {} {}",
                                ts,
                                channel,
                                id,
                                data_len,
                                bytes
                            );
                        }
                    }
                }
            }
            continue;
        }

        if status == XL_ERR_QUEUE_IS_EMPTY {
            thread::sleep(Duration::from_millis(5));
            continue;
        }

        unsafe {
            xlDeactivateChannel(port_handle, access_mask);
            xlClosePort(port_handle);
        }
        return Err(format!(
            "xlCanReceive failed: status {} ({})",
            status,
            xl_error_string(status)
        ));
    }

    unsafe {
        xlDeactivateChannel(port_handle, access_mask);
        xlClosePort(port_handle);
    }

    if let Some(file) = &mut log_file {
        if let CanLogFormat::Asc = log_format {
            let _ = writeln!(file, "End Triggerblock");
        }
    }

    println!("Stopped listening. Total frames captured: {}", frame_count);
    Ok(())
}

pub fn listen_can_all_connected(
    max_channels: u32,
    app_name: &str,
    interface_version: u32,
    duration_ms: Option<u64>,
    output_dir: Option<&str>,
    log_format: CanLogFormat,
) -> Result<(), String> {
    let mut usable_channels: Vec<u32> = Vec::new();

    for channel in 0..max_channels {
        match try_capture_can_on_channel(channel, 100, app_name, interface_version) {
            Ok(_) => {
                usable_channels.push(channel);
                println!("Detected usable channel {}", channel + 1);
            }
            Err(e) => {
                println!("Skipping channel {}: {}", channel + 1, e);
            }
        }
    }

    if usable_channels.is_empty() {
        return Err("No usable channels detected for capture".to_string());
    }

    let base_dir = output_dir.unwrap_or(".");
    fs::create_dir_all(base_dir)
        .map_err(|e| format!("Failed to create output directory '{}': {}", base_dir, e))?;

    println!(
        "Starting parallel capture on channels: {:?}",
        usable_channels
    );

    let mut handles = Vec::new();
    for channel in usable_channels {
        let app_name_owned = app_name.to_string();
        let file_stem = can_output_file_stem(channel);
        let file_name = match log_format {
            CanLogFormat::Asc => format!("{}.asc", file_stem),
            CanLogFormat::Text => format!("{}.log", file_stem),
        };
        let log_path = PathBuf::from(base_dir).join(file_name);
        let log_path_string = log_path.to_string_lossy().into_owned();

        handles.push(thread::spawn(move || {
            let result = listen_can_on_channel(
                channel,
                &app_name_owned,
                interface_version,
                duration_ms,
                Some(&log_path_string),
                log_format,
            );
            (channel, result)
        }));
    }

    let mut failures = 0u32;
    for handle in handles {
        match handle.join() {
            Ok((channel, Ok(()))) => {
                println!("Channel {} capture finished", channel + 1);
            }
            Ok((channel, Err(e))) => {
                failures += 1;
                println!("Channel {} capture failed: {}", channel + 1, e);
            }
            Err(_) => {
                failures += 1;
                println!("A channel capture thread panicked");
            }
        }
    }

    if failures > 0 {
        return Err(format!("{} channel capture(s) failed", failures));
    }

    Ok(())
}
