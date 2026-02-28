// CAN/ETH capture via vxlapi.dll (Vector hardware)
// This is a standalone component, not interfering with the main logger.
// Only loaded/used if enabled in the main app.

// --- Begin inlined vxlapi.dll FFI bindings ---
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong, c_void};

#[allow(non_camel_case_types)]
pub type XLstatus = c_uint;
#[allow(non_camel_case_types)]
pub type XLportHandle = c_uint;
#[allow(non_camel_case_types)]
pub type XLaccess = c_ulonglong;
#[allow(non_camel_case_types)]
pub type XLdriverConfig = c_void;

#[repr(C)]
pub struct XLcanRxEvent {
    pub tag: u32,
    pub chanIndex: u32,
    pub flags: u32,
    pub id: u32,
    pub dlc: u8,
    pub data: [u8; 8],
    pub reserved: [u8; 3],
    pub timeStamp: u64,
}

#[link(name = "vxlapi")]
unsafe extern "C" {
    pub fn xlOpenDriver() -> XLstatus;
    pub fn xlCloseDriver() -> XLstatus;
    pub fn xlGetDriverConfig(pDriverConfig: *mut XLdriverConfig) -> XLstatus;
    pub fn xlOpenPort(portHandle: *mut XLportHandle, appName: *const c_char, accessMask: XLaccess, permissionMask: XLaccess, rxQueueSize: u32, pPortHandle: *mut c_void, busType: u32) -> XLstatus;
    pub fn xlSetNotification(portHandle: XLportHandle, hwnd: *mut c_void, msg: u32) -> XLstatus;
    pub fn xlActivateChannel(portHandle: XLportHandle, accessMask: XLaccess, busType: u32, flags: u32) -> XLstatus;
    pub fn xlReceive(portHandle: XLportHandle, pEventCount: *mut u32, pEventList: *mut XLcanRxEvent) -> XLstatus;
    pub fn xlDeactivateChannel(portHandle: XLportHandle, accessMask: XLaccess) -> XLstatus;
    pub fn xlClosePort(portHandle: XLportHandle) -> XLstatus;
}
// --- End inlined vxlapi.dll FFI bindings ---
use std::ffi::{CString};
use std::ptr;

pub fn try_open_driver() -> Result<(), String> {
    let status = unsafe { xlOpenDriver() };
    if status == 0 {
        Ok(())
    } else {
        Err(format!("xlOpenDriver failed: status {}", status))
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
    // Minimal CAN capture example (single channel, no error recovery)
    use std::mem::{zeroed, MaybeUninit};
    use std::ffi::CString;
    let app_name = CString::new("RustApp").unwrap();
    let mut port_handle: XLportHandle = 0;
    let access_mask: XLaccess = 1; // channel 0
    let permission_mask: XLaccess = 1;
    let rx_queue_size = 128u32;
    let bus_type = 1u32; // 1 = CAN

    // Open port
    let status = unsafe {
        xlOpenPort(&mut port_handle as *mut _, app_name.as_ptr(), access_mask, permission_mask, rx_queue_size, std::ptr::null_mut(), bus_type)
    };
    if status != 0 {
        return Err(format!("xlOpenPort failed: status {}", status));
    }

    // Activate channel
    let status = unsafe { xlActivateChannel(port_handle, access_mask, bus_type, 0) };
    if status != 0 {
        unsafe { xlClosePort(port_handle) };
        return Err(format!("xlActivateChannel failed: status {}", status));
    }

    // Receive one CAN event (non-blocking, demo only)
    let mut event_count: u32 = 1;
    let mut event = MaybeUninit::<XLcanRxEvent>::zeroed();
    let status = unsafe {
        xlReceive(port_handle, &mut event_count as *mut _, event.as_mut_ptr())
    };
    if status == 0 && event_count > 0 {
        let event = unsafe { event.assume_init() };
        println!("Received CAN id=0x{:X} dlc={} data={:02X?}", event.id, event.dlc, &event.data[..event.dlc as usize]);
    } else {
        println!("No CAN event received or error: status {} count {}", status, event_count);
    }

    // Deactivate and close
    unsafe {
        xlDeactivateChannel(port_handle, access_mask);
        xlClosePort(port_handle);
    }
    Ok(())
}
