// Low-level vxlapi.dll FFI bindings for CAN/ETH capture
// This file is for advanced/complete vxlapi function coverage
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
extern "C" {
    pub fn xlOpenDriver() -> XLstatus;
    pub fn xlCloseDriver() -> XLstatus;
    pub fn xlGetDriverConfig(pDriverConfig: *mut XLdriverConfig) -> XLstatus;
    pub fn xlOpenPort(portHandle: *mut XLportHandle, appName: *const c_char, accessMask: XLaccess, permissionMask: *mut XLaccess, rxQueueSize: u32, xlInterfaceVersion: u32, busType: u32) -> XLstatus;
    pub fn xlSetNotification(portHandle: XLportHandle, hwnd: *mut c_void, msg: u32) -> XLstatus;
    pub fn xlActivateChannel(portHandle: XLportHandle, accessMask: XLaccess, busType: u32, flags: u32) -> XLstatus;
    pub fn xlReceive(portHandle: XLportHandle, pEventCount: *mut u32, pEventList: *mut XLcanRxEvent) -> XLstatus;
    pub fn xlDeactivateChannel(portHandle: XLportHandle, accessMask: XLaccess) -> XLstatus;
    pub fn xlClosePort(portHandle: XLportHandle) -> XLstatus;
}
