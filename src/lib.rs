use rayon::prelude::*;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::Metadata;
use std::path::Path;

mod ffi;

pub struct DiskItem {
    pub name: OsString,
    pub disk_size: u64,
    pub children: Option<Vec<DiskItem>>,
}

trait ApparentSize {
    fn size(&self, apparent: bool, path: &Path) -> Result<u64, Box<Error>>;
}

impl ApparentSize for Metadata {
    #[cfg(unix)]
    fn size(&self, apparent: bool, _path: &Path) -> Result<u64, Box<Error>> {
        if apparent {
            use std::os::unix::fs::MetadataExt;
            Ok(self.blocks() * 512)
        } else {
            Ok(self.len())
        }
    }

    #[cfg(windows)]
    fn size(&self, apparent: bool, path: &Path) -> Result<u64, Box<Error>> {
        if apparent {
            ffi::compressed_size(path)
        } else {
            Ok(self.len())
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
                disk_size: file_info.size(apparent, path)?,
                children: None,
            })
        }
    }
}
