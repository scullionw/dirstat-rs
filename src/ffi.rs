#![cfg(windows)]

use std::io;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::DWORD;
use winapi::um::fileapi::FILE_ATTRIBUTE_TAG_INFO;
use winapi::um::fileapi::FILE_COMPRESSION_INFO;
use winapi::um::fileapi::FILE_ID_INFO;
use winapi::um::fileapi::FILE_STANDARD_INFO;
use winapi::um::fileapi::FILE_STORAGE_INFO;
use winapi::um::minwinbase::FileCompressionInfo;
use winapi::um::minwinbase::FileIdInfo;
use winapi::um::minwinbase::FileStandardInfo;
use winapi::um::minwinbase::FileStorageInfo;
use winapi::um::minwinbase::FILE_INFO_BY_HANDLE_CLASS;
use winapi::um::winbase::GetFileInformationByHandleEx;
use winapi_util::AsHandleRef;
use winapi_util::Handle;

#[cfg(test)]
use std::path::Path;

/// Provides mapping from structs of file information to corresponding [FILE_INFO_BY_HANDLE_CLASS](winapi::um::minwinbase::FILE_INFO_BY_HANDLE_CLASS) constant[^info_class].
///
///
/// List of supported classes supported by Windows can be found on
/// [MSDN](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex#remarks)
///
/// Be aware that classes with name in format `**RestartInfo` are paired with same name wituhout Restart info,
/// and are intended to be used with enumeration logic. They won't be covered by THIS trait as they require special handling,
/// and such structs has unknown size. They potentially can be used in future versions to speedup size calculation as they
/// can be used to accuire list of files+their sizes in single call with directorie's HANDLE.
///
/// This instances should be used with [get_file_information_by_handle_ex] function
///
/// [^info_class]: See: [FILE_INFO_BY_HANDLE_CLASS enumeration on MSDN](https://learn.microsoft.com/en-us/windows/win32/api/minwinbase/ne-minwinbase-file_info_by_handle_class)
pub trait FileInfoTrait: Default + Sized {
    const CLASS: FILE_INFO_BY_HANDLE_CLASS;
}

impl FileInfoTrait for FILE_STANDARD_INFO {
    const CLASS: FILE_INFO_BY_HANDLE_CLASS = FileStandardInfo;
}

impl FileInfoTrait for FILE_COMPRESSION_INFO {
    const CLASS: FILE_INFO_BY_HANDLE_CLASS = FileCompressionInfo;
}

impl FileInfoTrait for FILE_STORAGE_INFO {
    const CLASS: FILE_INFO_BY_HANDLE_CLASS = FileStorageInfo;
}

impl FileInfoTrait for FILE_ID_INFO {
    const CLASS: FILE_INFO_BY_HANDLE_CLASS = FileIdInfo;
}

/// If we decide not following symlinks, we would need this, and FILE_ATTRIBUTE_REPARSE_POINT
impl FileInfoTrait for FILE_ATTRIBUTE_TAG_INFO {
    const CLASS: FILE_INFO_BY_HANDLE_CLASS = FileIdInfo;
}

// https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntquerydirectoryfile
// https://www.winehq.org/pipermail/wine-cvs/2015-May/106715.html
// https://github.com/MicrosoftDocs/sdk-api/blob/docs/sdk-api-src/content/minwinbase/ne-minwinbase-file_info_by_handle_class.md#-field-fileidbothdirectoryinfo

/// Gets file information by handle.
/// It can be various types of information.
///
/// List of supported types by Windows OS at time of writing is:
/// * FILE_BASIC_INFO
/// * FILE_STANDARD_INFO
/// * FILE_NAME_INFO
/// * FILE_STREAM_INFO -- most likely won't work as expected, as this is also expecting to return
/// array of unsized structs. But this particular one don't support integrating,
/// and we need to provide whole size upfront. If we want to use this, we would need to try with
/// particular buffer size, and if it's not enough -- reallocate buffer and try again.
/// * FILE_COMPRESSION_INFO
/// * FILE_ATTRIBUTE_TAG_INFO
/// * FILE_ID_BOTH_DIR_INFO -- not supported by this function.
/// * FILE_REMOTE_PROTOCOL_INFO
/// * FILE_FULL_DIR_INFO -- not supported by this function.
/// * FILE_STORAGE_INFO
/// * FILE_ALIGNMENT_INFO
/// * FILE_ID_INFO
/// * FILE_ID_EXTD_DIR_INFO -- not supported by this function.
///
/// To use this structs you should implement [FileInfoTrait] for them.
/// It's not supported for types with "not supported" remark, and this function should not be used for them.
///
/// This method is responsible for calling [GetFileInformationByHandleEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex) function.
///
pub fn get_file_information_by_handle_ex<T: FileInfoTrait>(
    handle: &Handle,
) -> Result<T, io::Error> {
    let mut buf = T::default();

    let res = unsafe {
        GetFileInformationByHandleEx(
            handle.as_raw(),
            T::CLASS,
            &mut buf as *mut _ as *mut c_void,
            std::mem::size_of_val(&buf) as DWORD,
        )
    };

    if res != 0 {
        Result::Ok(buf)
    } else {
        Result::Err(io::Error::last_os_error())
    }
}

/// Extracts rust-native long value from Windows LARGE_INTEGER union.
///
/// That union is basically way to get access to Low-word and High-word of i64.
/// Most likely for compatibility with win16. It was checked to work with Rust on 32bit target (i686).
pub fn read_large_integer(size: winapi::um::winnt::LARGE_INTEGER) -> i64 {
    // SAFETY: this is marked as unsafe, because it is access to union fields. But this particular union consists of just integer numbers
    // of different sizes, so any bit pattern is valid.
    let size = *unsafe { size.QuadPart() };
    size
}

/// Sets file's special attribute "Compressed" to on or off.
#[cfg(test)]
pub fn set_file_compression<P: AsRef<Path>>(path: P, compress: bool) -> Result<(), io::Error> {
    use std::fs::File;
    use std::ptr::null_mut;

    use winapi::um::ioapiset::DeviceIoControl;
    use winapi::um::winioctl::FSCTL_SET_COMPRESSION;
    use winapi::{
        shared::minwindef::{LPVOID, USHORT},
        um::winnt::{COMPRESSION_FORMAT_LZNT1, COMPRESSION_FORMAT_NONE},
    };

    // see documentation at https://learn.microsoft.com/en-us/previous-versions/windows/embedded/ms890601(v=msdn.10)
    // and https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-fsctl_set_compression
    let handle = Handle::from_file(File::options().write(true).read(true).open(path)?);

    // mut, coz WinAPI rquires mut pointer
    let mut compression_format: USHORT = if compress {
        COMPRESSION_FORMAT_LZNT1
    } else {
        COMPRESSION_FORMAT_NONE
    };

    unsafe {
        let res = DeviceIoControl(
            handle.as_raw(),
            FSCTL_SET_COMPRESSION,
            &mut compression_format as *mut _ as LPVOID,
            std::mem::size_of_val(&compression_format) as u32,
            null_mut(),
            0,
            null_mut(),
            null_mut(),
        );
        if res != 0 {
            Ok(())
        } else {
            Result::Err(io::Error::last_os_error())
        }
    }
}
