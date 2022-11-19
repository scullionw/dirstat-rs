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
    /// Analyzes provided path and returns tree structure of analyzed DiskItems
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
                // TODO: we can provide size for directories. Linux reports it, and `du` is actually using it in it's calculations
            })
        } else {
            let size = if apparent {
                md.blocks() * 512
            } else {
                md.len()
            };
            Ok(FileInfo::File {
                size,
                volume_id: md.dev(),
            })
        }
    }

    #[cfg(windows)]
    pub fn from_path(path: &Path, apparent: bool) -> Result<Self, Box<dyn Error>> {
        use crate::ffi::get_file_information_by_handle_ex;
        use std::convert::TryInto;
        use winapi::um::fileapi::FILE_ID_INFO;
        use winapi::um::fileapi::FILE_STANDARD_INFO;
        use winapi::um::winnt::LARGE_INTEGER;
        use winapi_util::Handle;

        /*
            Windows Implemntation notice.

            File size is tricky on windows.
            Here is article by Raymond Chen from Microsoft https://devblogs.microsoft.com/oldnewthing/20160427-00/?p=93365

            Quot from there:
            ```
            The algorithm for “Size on disk” is as follows:

            * If the file is sparse, then report the number of non-sparse bytes.
            * If the file is compressed, then report the compressed size. The compressed size may be less than a full sector.
            * If the file is neither sparse nor compressed, then report the nominal file size, rounded up to the nearest cluster.

            Starting in Windows 8.1, the Size on disk calculation includes the sizes of alternate data streams
            and sort-of-kind-of tries to guess which streams could be stored in the MFT
            and not count them toward the size on disk. (Even though they really are on disk.
            I mean, if they’re not on disk, then where are they?)
            ```

            From my research and observations:

            Win API Reports size on disk wierdly for small files.
            AllocationSize field of structs can be not dividable by FS cluster size.
            Also AllocationSize could be one byte bigger than size reported by GetCompressedFileSize function,
            or value obtained by FILE_COMPRESSION_INFO struct.

            But when we open properties windows for such file -- explorer would report size 0.
            That indicates that file is actually stored inside directory.
            We also can get size of directory on disk using this APIs.

            In my opiniopn (Bohdan Mart) perfect soulition would be to detect somehow that file is stored inline,
            and for such file also report 0 size. And add own size of directory for final result.
            Current implemetation is just adding reported file physical syze if -a flag is provided.
            Which is consistent with versions of dirstat-rs 0.3.8 and earlier.

            Also I have noticed that we can read file sizes in bulk from directory handle (fd).
            Basically we pass in dirrectory handle and receive iterator of FILE_FULL_DIR_INFO,
            which contrains both AllocationSize and EndOfFile.
            If this can benefit performance is needed to be tested.

            Second problem are alternate data streams. On Windows each file can have multiple data streams,
            and main stream called $DATA. Data streams can be opened if we try open file with name ending
            in `:<streamName>` like "some_file.txt:stream1".

            Upon experimentation it is clear that windows explorer is calculating size of data streams as well.

            On windows each stream can have any length, even several TB. On linux therea re similar feature,
            clled *extended attributes*, but it have limited size.

            It would be nice for dirstat to get size of alternate datastreams. Unfortuantely it is not compatible
            with FILE_FULL_DIR_INFO, so it should be tested, if taht optimisation is actually needed.
            perhaps calcualting alt DS streams size can be optional flag, to maximize performance.

            Possible future work items:
            1. Check if getting list of files and their sizes in bulk would be benficial.
            2. Try to mimic windows explorer algorithm to calculate file size.
            3. Get size for alternate data streams
                (perhaps we don't need to get regular size, as it is reported with datastreams as well)

            More info in SO question https://stackoverflow.com/questions/51033508/how-do-i-get-the-size-of-file-in-disk

            Some playground I've used to experiment with API https://gist.github.com/Mart-Bogdan/bda2995621911254f73f80d157f07622
        */

        let h = Handle::from_path_any(path)?;
        let std_info: FILE_STANDARD_INFO = get_file_information_by_handle_ex(&h)?;
        // That's unfortunate that we have to make second syscall just to know volume serial number
        let id_info: FILE_ID_INFO = get_file_information_by_handle_ex(&h)?;
        // If we decide to skip symlinks, we would also need FILE_ATTRIBUTE_TAG_INFO struct.

        if std_info.Directory != 0 {
            Ok(FileInfo::Directory {
                volume_id: id_info.VolumeSerialNumber,
                // TODO file size is actually provided for directories. We can use it to provide more precise info.
            })
        } else {
            let size: LARGE_INTEGER = if apparent {
                std_info.AllocationSize
            } else {
                std_info.EndOfFile
            };

            let size = ffi::read_large_integer(size);

            Ok(FileInfo::File {
                size: size.try_into()?, // it is i64 on windows
                volume_id: id_info.VolumeSerialNumber,
            })
        }
    }
}
