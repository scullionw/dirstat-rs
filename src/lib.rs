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
    fn size(&self, apparent: bool, path: &Path) -> Result<u64, Box<dyn Error>>;
}

impl ApparentSize for Metadata {
    #[cfg(unix)]
    fn size(&self, apparent: bool, _path: &Path) -> Result<u64, Box<dyn Error>> {
        if apparent {
            use std::os::unix::fs::MetadataExt;
            Ok(self.blocks() * 512)
        } else {
            Ok(self.len())
        }
    }

    #[cfg(windows)]
    fn size(&self, apparent: bool, path: &Path) -> Result<u64, Box<dyn Error>> {
        if apparent {
            ffi::compressed_size(path)
        } else {
            Ok(self.len())
        }
    }
}

impl DiskItem {
    pub fn from_analyze(
        path: &Path,
        apparent: bool,
        root_dev: u64,
    ) -> Result<Self, Box<dyn Error>> {
        let name = path.file_name().unwrap_or(&OsStr::new(".")).to_os_string();
        let file_info = path.symlink_metadata()?;

        if file_info.is_dir() {
            let sub_entries = fs::read_dir(path)?
                .filter_map(Result::ok)
                .collect::<Vec<_>>();

            let mut sub_items = sub_entries
                .par_iter()
                .filter_map(|entry| match device_num(&entry.path()) {
                    Ok(id) if id == root_dev => {
                        DiskItem::from_analyze(&entry.path(), apparent, root_dev).ok()
                    }
                    _ => None,
                })
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

#[cfg(unix)]
pub fn device_num<P: AsRef<Path>>(path: P) -> std::io::Result<u64> {
    use std::os::unix::fs::MetadataExt;

    path.as_ref().metadata().map(|md| md.dev())
}

#[cfg(windows)]
pub fn device_num<P: AsRef<Path>>(path: P) -> std::io::Result<u64> {
    use winapi_util::{file, Handle};

    let h = Handle::from_path_any(path)?;
    file::information(h).map(|info| info.volume_serial_number())
}
