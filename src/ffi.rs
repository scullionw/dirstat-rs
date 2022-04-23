#![cfg(windows)]

use std::error::Error;
use std::io;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use winapi::shared::winerror::NO_ERROR;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::fileapi::GetCompressedFileSizeW;
use winapi::um::fileapi::INVALID_FILE_SIZE;

pub fn compressed_size(path: &Path) -> Result<u64, Box<dyn Error>> {
    let wide = path_to_u16s(path);
    let mut high: u32 = 0;

    // TODO: Deal with max path size
    let low = unsafe { GetCompressedFileSizeW(wide.as_ptr(), &mut high) };

    if low == INVALID_FILE_SIZE {
        let err = get_last_error();
        if err != NO_ERROR {
            return Err(io::Error::last_os_error().into());
        }
    }

    Ok(u64::from(high) << 32 | u64::from(low))
}

/// inspired by [fn maybe_verbatim(path: &Path)](https://github.com/rust-lang/rust/blob/1f4681ad7a132755452c32a987ad0f0d075aa6aa/library/std/src/sys/windows/path.rs#L170)
/// But function from std is calling winapi GetFullPathNameW in case if path is longer than 248.
/// We are more optimistic and expect all path being absolute, so no API calls from this function.
fn path_to_u16s(path: &Path) -> Vec<u16> {
    // Normally the MAX_PATH is 260 UTF-16 code units (including the NULL).
    // However, for APIs such as CreateDirectory[1], the limit is 248.
    //
    // [1]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectorya#parameters
    const LEGACY_MAX_PATH: usize = 248;
    // UTF-16 encoded code points, used in parsing and building UTF-16 paths.
    // All of these are in the ASCII range so they can be cast directly to `u16`.
    const SEP: u16 = b'\\' as _;
    const QUERY: u16 = b'?' as _;
    const U: u16 = b'U' as _;
    const N: u16 = b'N' as _;
    const C: u16 = b'C' as _;
    // \\?\
    const VERBATIM_PREFIX: &[u16] = &[SEP, SEP, QUERY, SEP];
    // \??\
    const NT_PREFIX: &[u16] = &[SEP, QUERY, QUERY, SEP];
    // \\?\UNC\
    const UNC_PREFIX: &[u16] = &[SEP, SEP, QUERY, SEP, U, N, C, SEP];
    // \\
    const NETWORK_PREFIX: &[u16] = &[SEP, SEP];

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(once(0)).collect();
    // don't need to do anything if path is small enaught.
    if wide.len() < LEGACY_MAX_PATH {
        return wide;
    }

    if wide.starts_with(VERBATIM_PREFIX) || wide.starts_with(NT_PREFIX) {
        return wide;
    }

    if wide.starts_with(NETWORK_PREFIX) {
        // network path from SMB
        let mut tmp = Vec::from(UNC_PREFIX);
        tmp.extend(&wide[2..]);
        return tmp;
    } else {
        // if we came here, we aren't using network drive, so just prepend File namespace prefix
        let mut tmp = Vec::from(VERBATIM_PREFIX);
        tmp.extend(wide);
        return tmp;
    }
}

fn get_last_error() -> u32 {
    unsafe { GetLastError() }
}
