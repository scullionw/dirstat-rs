use rayon::prelude::*;
use serde::Serialize;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

mod ffi;

#[cfg(test)]
mod tests;

#[derive(Serialize)]
pub struct DiskItem {
    pub name: String,
    pub disk_size: u64,
    pub children: Option<Vec<DiskItem>>,
}

impl DiskItem {
    pub fn from_analyze(
        path: &Path,
        apparent: bool,
        root_dev: u64,
    ) -> Result<Self, Box<dyn Error>> {
        let name = path
            .file_name()
            .unwrap_or(&OsStr::new("."))
            .to_string_lossy()
            .to_string();

        let file_info = FileInfo::from_path(path, apparent)?;

        match file_info {
            FileInfo::Directory { volume_id } => {
                if volume_id != root_dev {
                    return Err("Filesystem boundary crossed".into());
                }

                let sub_entries = fs::read_dir(path)?
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>();

                let mut sub_items = sub_entries
                    .par_iter()
                    .filter_map(|entry| {
                        DiskItem::from_analyze(&entry.path(), apparent, root_dev).ok()
                    })
                    .collect::<Vec<_>>();

                sub_items.sort_unstable_by(|a, b| a.disk_size.cmp(&b.disk_size).reverse());

                Ok(DiskItem {
                    name,
                    disk_size: sub_items.iter().map(|di| di.disk_size).sum(),
                    children: Some(sub_items),
                })
            }
            FileInfo::File { size, .. } => Ok(DiskItem {
                name,
                disk_size: size,
                children: None,
            }),
        }
    }
}

pub enum FileInfo {
    File { size: u64, volume_id: u64 },
    Directory { volume_id: u64 },
}

impl FileInfo {
    #[cfg(unix)]
    pub fn from_path(path: &Path, apparent: bool) -> Result<Self, Box<dyn Error>> {
        use std::os::unix::fs::MetadataExt;

        let md = path.symlink_metadata()?;
        if md.is_dir() {
            Ok(FileInfo::Directory {
                volume_id: md.dev(),
            })
        } else {
            let size = if apparent {
                md.len()
            } else {
                md.blocks() * 512
            };
            Ok(FileInfo::File {
                size,
                volume_id: md.dev(),
            })
        }
    }

    #[cfg(windows)]
    pub fn from_path(path: &Path, apparent: bool) -> Result<Self, Box<dyn Error>> {
        use winapi_util::{file, Handle};
        const FILE_ATTRIBUTE_DIRECTORY: u64 = 0x10;

        let h = Handle::from_path_any(path)?;
        let md = file::information(h)?;

        if md.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0 {
            Ok(FileInfo::Directory {
                volume_id: md.volume_serial_number(),
            })
        } else {
            let size = if apparent {
                md.file_size()
            } else {
                ffi::compressed_size(path)?
            };
            Ok(FileInfo::File {
                size,
                volume_id: md.volume_serial_number(),
            })
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::FileInfo;
    use std::error::Error;
    use std::fs::{self, File};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn apparent_size_uses_logical_file_length() -> Result<(), Box<dyn Error>> {
        let dir = std::env::temp_dir().join(format!(
            "dirstat-rs-{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir(&dir)?;
        let path = dir.join("sparse.bin");
        let file = File::create(&path)?;
        let sparse_len = 1024 * 1024;
        file.set_len(sparse_len)?;
        drop(file);

        let apparent_size = match FileInfo::from_path(&path, true)? {
            FileInfo::File { size, .. } => size,
            FileInfo::Directory { .. } => panic!("test path should be a file"),
        };
        let disk_size = match FileInfo::from_path(&path, false)? {
            FileInfo::File { size, .. } => size,
            FileInfo::Directory { .. } => panic!("test path should be a file"),
        };

        fs::remove_file(&path)?;
        fs::remove_dir(&dir)?;

        assert_eq!(apparent_size, sparse_len);
        assert!(disk_size <= apparent_size);
        Ok(())
    }
}
