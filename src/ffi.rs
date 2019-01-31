#![cfg(windows)]

use std::error::Error;
use std::io;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::null_mut;
use winapi::shared::winerror::NO_ERROR;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::fileapi::GetCompressedFileSizeW;
use winapi::um::fileapi::INVALID_FILE_SIZE;

pub fn compressed_size(path: &Path) -> Result<u64, Box<dyn Error>> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(once(0)).collect();
    let high: *mut u32 = null_mut();

    // TODO: Deal with max path size
    let low = unsafe { GetCompressedFileSizeW(wide.as_ptr(), high) };

    if low == INVALID_FILE_SIZE {
        let err = get_last_error();
        if err != NO_ERROR {
            return Err(io::Error::last_os_error().into());
        }
    }

    if high.is_null() {
        Ok(low as u64)
    } else {
        let high = unsafe { *high };
        Ok((low as u64) | (high as u64) << 32)
    }
}

fn get_last_error() -> u32 {
    unsafe { GetLastError() }
}
