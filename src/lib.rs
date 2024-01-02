use std::ffi::{CStr, CString};

use ffi::{do_command, error_json, FFIError};
use libc::c_char;
use serde_json::Value;

pub mod api;
pub mod blend;
pub mod db;

pub mod ffi;

/// # Safety
///
/// this public function might dereference a raw pointer
#[no_mangle]
pub unsafe extern "C" fn call_command(c_string_ptr: *const c_char) -> *mut c_char {
    let bytes = unsafe { CStr::from_ptr(c_string_ptr).to_bytes() };

    let string = std::str::from_utf8(bytes).unwrap();
    let data: Value = serde_json::from_str(string).unwrap();

    let result = do_command(data);

    let json_string = match result {
        Ok(str) => str,
        Err(e) => {
            serde_json::to_string(&error_json(FFIError::InternalError(format!("{e}")))).unwrap()
        }
    };

    let c_string = CString::new(json_string).unwrap();

    c_string.into_raw()
}

/// # Safety
///
/// this public function might dereference a raw pointer
#[no_mangle]
pub unsafe extern "C" fn free_command(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    let _ = CStr::from_ptr(ptr);
}
