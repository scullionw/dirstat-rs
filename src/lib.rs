use rayon::prelude::*;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;

pub struct DiskItem {
    pub name: OsString,
    pub disk_size: u64,
    pub children: Option<Vec<DiskItem>>,
}

impl DiskItem {
    pub fn from_analyze(path: &Path) -> Result<Self, Box<Error>> {
        let name = path.file_name().unwrap_or(&OsStr::new(".")).to_os_string();
        let file_info = path.symlink_metadata()?;

        if file_info.is_dir() {
            let sub_entries = fs::read_dir(path)?
                .filter_map(Result::ok)
                .collect::<Vec<_>>();

            let mut sub_items = sub_entries
                .par_iter()
                .filter_map(|entry| DiskItem::from_analyze(&entry.path()).ok())
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
                disk_size: file_info.len(),
                children: None,
            })
        }
    }
}
