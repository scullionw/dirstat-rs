use rayon::prelude::*;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::Metadata;
use std::path::Path;

pub struct DiskItem {
    pub name: OsString,
    pub disk_size: u64,
    pub children: Option<Vec<DiskItem>>,
}


trait ApparentSize {
    fn size(&self, apparent: bool, path: &Path) -> u64;
}

impl ApparentSize for Metadata {
    #[cfg(unix)]
    fn size(&self, apparent: bool, _path: &Path) -> u64 {
        if apparent {
            use std::os::unix::fs::MetadataExt;
            self.blocks() * 512
        } else {
            self.len()
        }
    }

    #[cfg(windows)]
    fn size(&self, apparent: bool, path: &Path) -> u64 {
        if apparent {
            use winapi::um::fileapi::GetCompressedFileSizeW;
            use std::ffi::OsStr;
            use std::iter::once;
            use std::os::windows::ffi::OsStrExt;
            use std::ptr::null_mut;
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain(once(0)).collect();
            let high: *mut u32 = null_mut();
            let low = unsafe { GetCompressedFileSizeW(wide.as_ptr(), high) };
            let high = unsafe { *high as u64 };
            let total= (low as u64) | (high << 32);
            total
        } else {
            self.len()
        }
    }
}


impl DiskItem {
    pub fn from_analyze(path: &Path, apparent: bool) -> Result<Self, Box<Error>> {
        let name = path.file_name().unwrap_or(&OsStr::new(".")).to_os_string();
        let file_info = path.symlink_metadata()?;

        if file_info.is_dir() {
            let sub_entries = fs::read_dir(path)?
                .filter_map(Result::ok)
                .collect::<Vec<_>>();

            let mut sub_items = sub_entries
                .par_iter()
                .filter_map(|entry| DiskItem::from_analyze(&entry.path(), apparent).ok())
                .collect::<Vec<_>>();

            sub_items.sort_unstable_by_key(|di| di.disk_size);

            Ok(DiskItem {
                name,
                disk_size: sub_items.iter().map(|di| di.disk_size).sum(),
                children: Some(sub_items),
            })
        } else {
            Ok(DiskItem {
                name,
                disk_size: file_info.size(apparent, path),
                children: None,
            })
        }
    }
}
