//! Be aware that tests are run in parallel. For this reason we must be sure to use
//! separate dirs for different cases.

use crate::{DiskItem, FileInfo};
// warn: don't remove `as &str` after macro invocation.
// It breaks type checker in Intellij Rust IDE
use const_format::concatcp;
use std::fs::File;
use std::io::Write;
use std::panic;
use std::path::Path;

// be aware that rust runs tests in parallel, so tests should use different dirs

const TEST_DATA_DIR: &str = "./test-data/";

const LONG_PATH_DIR: &str = "long-path/";
//noinspection SpellCheckingInspection
const PATH_1: &str = "lll1/llllllll/llllllllllllllll/llllllllllllll/lllllllllllll/oooooo\
oooooooo/oooooooooooooooo/nnnnnnnnn/nnnnnnnnnn/nnnnnnnn/nnnnnn/gggggggggg/p/a/tttt\
tttttttttt/ttttttttttt/ttttttttttttttt/ttttttttt/tttthhh/2222222222/22222222222/222222222222/\
3333333333333/33333333/33333333333/33333333333/333333333/33333333/44444444/44444444444444444/\
5555555/55555555555/55555555/5555555555/5555555/5555555/555555/555555555/66666666666666666666/\
77777777/7777777777/7777777777777/77777777777/7777777777/77777777/7777777/77777777/8888888888/\
99999999/999999/99999999/99999999999/99999999/999999999/9999999999/";

const PATH_1_FULL: &str = concatcp!(TEST_DATA_DIR, LONG_PATH_DIR, PATH_1) as &str;
//noinspection SpellCheckingInspection
const PATH_2: &str = "lll2/llllllll/llllllllllllllll/llllllllllllll/lllllllllllll/oooooo\
oooooooo/oooooooooooooooo/nnnnnnnnn/nnnnnnnnnn/nnnnnnnn/nnnnnn/gggggggggg/p/a/tttt\
tttttttttt/ttttttttttt/ttttttttttttttt/ttttttttt/tttthhh/2222222222/22222222222/222222222222/\
3333333333333/33333333/33333333333/33333333333/333333333/33333333/44444444/44444444444444444/\
5555555/55555555555/55555555/5555555555/5555555/5555555/555555/555555555/66666666666666666666/\
77777777/7777777777/7777777777777/77777777777/7777777777/77777777/7777777/77777777/8888888888/\
99999999/999999/99999999/99999999999/99999999/999999999/9999999999/";

const PATH_2_FULL: &str = concatcp!(TEST_DATA_DIR, LONG_PATH_DIR, PATH_2) as &str;

const TEST_PRE_CREATED_DIR: &str = concatcp!(TEST_DATA_DIR, "pre-created/");

#[test]
fn test_max_path() {
    // do not rename it into `_` it would cause immediate destrucion after creation
    let _guard = CleanUpGuard {
        path: concatcp!(TEST_DATA_DIR, LONG_PATH_DIR) as &str,
    };

    // Given
    create_dir(PATH_1_FULL);
    create_dir(PATH_2_FULL);
    create_file(&concatcp!(PATH_1_FULL, "file.bin"), 4096);
    create_file(&concatcp!(PATH_2_FULL, "file.bin"), 8192);

    // When
    let test_path = Path::new(concatcp!(TEST_DATA_DIR, LONG_PATH_DIR) as &str);
    let result = FileInfo::from_path(test_path, true);

    // Then
    if let Result::Ok(FileInfo::Directory { volume_id }) = result {
        let result = DiskItem::from_analyze(test_path, true, volume_id);
        let result = result.expect("Must collect data");
        assert_eq!(result.disk_size, 4096 + 8192);
        let children = result.children.unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].disk_size, 8192);
        assert_eq!(children[1].disk_size, 4096);
    } else {
        panic!("Can not get file info");
    }
}

#[test]
fn test_files_logical_size() {
    // TODO windows explorer reports 107564. and actual sum of sizes is 107564 as well
    // assert_size(TEST_PRE_CREATED_DIR, false, 123943);
    assert_size(TEST_PRE_CREATED_DIR, false, 107564);

    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b23_rand"), false, 23);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b23_zero"), false, 23);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4000_rand"), false, 4000);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4000_zero"), false, 4000);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4096_rand"), false, 4096);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4096_zero"), false, 4096);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b512_rand"), false, 512);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b512_zero"), false, 512);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b8000_rand"), false, 8000);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b8192_rand"), false, 8192);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b8192_zero"), false, 8192);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "rand_1000"), false, 1000);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "text1.txt"), false, 2088);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "text2.txt"), false, 9048);
}
#[test]
fn test_files_physical_size() {
    // Can't test top dir, as compressed files would mess the picture

    // following are windows quirks/optimisations
    if cfg!(windows) {
        assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b23_rand"), true, 24);
        assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b23_zero"), true, 24);
        assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b512_rand"), true, 512);
        assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b512_zero"), true, 512);
    } else {
        // TODO this is really FS dependant. On WSL and ntfs it all would be 0. With Ext4 it would be 4096
        // either add FS specific logic, or don't assert this. I guss second option, as otherwise tests
        // aren't reproducible.

        // assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b23_rand"), true, 0);
        // assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b23_zero"), true, 0);
        // assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b512_rand"), true, 0);
        // assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b512_zero"), true, 0);
    }

    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4000_rand"), true, 4096);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4000_zero"), true, 4096);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4096_rand"), true, 4096);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4096_zero"), true, 4096);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b8000_rand"), true, 8192);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b8192_rand"), true, 8192);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b8192_zero"), true, 8192);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "rand_1000"), true, 4096);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "text1.txt"), true, 4096);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "text2.txt"), true, 12288);
}
#[test]
#[cfg(windows)] // isn't supported on Unix (Theoretically possible on btrfs)
fn test_compressed_files_physical_size() {
    prepare_files_compression().unwrap();

    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b23_rand_c"), true, 24);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b23_zero_c"), true, 24);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b512_rand_c"), true, 512);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b512_zero_c"), true, 512);

    // Unreproducible: my Win10 -- 8192; WinServer(github) -- 4096
    //assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4000_rand_c"), true, 8192);
    //assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4000_zero_c"), true, 0);
    // CI assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4096_rand_c"), true, 8192);
    // CI assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b4096_zero_c"), true, 0);
    // assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b8000_rand_c"), true, 12288);
    // assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b8192_rand_c"), true, 12288);
    // CI assert_size(concatcp!(TEST_PRE_CREATED_DIR, "b8192_zero_c"), true, 0);
    // assert_size(concatcp!(TEST_PRE_CREATED_DIR, "rand_1000_c"), true, 4096);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "text1_c.txt"), true, 4096);
    assert_size(concatcp!(TEST_PRE_CREATED_DIR, "text2_c.txt"), true, 4096);
}

#[allow(non_snake_case)]
#[test]
fn test_file_size_8KiB() {
    const DIR: &str = concatcp!(TEST_DATA_DIR, "test_file_size/") as &str;
    // do not rename it into `_` it would cause immediate destrucion after creation
    let _guard = CleanUpGuard { path: DIR };

    // Given
    // Such sizes is selected to be close to filesystem sector size, and to be maximally universal
    // event for FS-es with sector as bif as 8KiB
    create_file(&concatcp!(DIR, "foo/file.bin") as &str, 8192);
    create_file(&concatcp!(DIR, "bar/file.bin") as &str, 8192 - 5);

    // When calculating with apparent size
    let test_path = Path::new(DIR);
    let result = FileInfo::from_path(test_path, true);

    // Then
    if let Result::Ok(FileInfo::Directory { volume_id }) = result {
        let result = DiskItem::from_analyze(test_path, true, volume_id);
        let result = result.expect("Must collect data");
        assert_eq!(result.disk_size, 8192 + 8192);
        let children = result.children.unwrap();
        assert_eq!(children.len(), 2);
        // Both dirs should be rounded to sector size
        assert_eq!(children[0].disk_size, 8192);
        assert_eq!(children[1].disk_size, 8192);
    } else {
        panic!("Can not get file info");
    }

    // When calculating withOUT apparent size
    let result = FileInfo::from_path(test_path, false);

    // Then
    if let Result::Ok(FileInfo::Directory { volume_id }) = result {
        let result = DiskItem::from_analyze(test_path, false, volume_id);
        let result = result.expect("Must collect data");
        assert_eq!(result.disk_size, 8192 + 8192 - 5);
        let children = result.children.unwrap();
        assert_eq!(children.len(), 2);
        // Both dirs should be rounded to sector size
        assert_eq!(children[0].disk_size, 8192);
        assert_eq!(children[1].disk_size, 8192 - 5);
    } else {
        panic!("Can not get file info");
    }
}

// Helper functions and cleanup code goes next

fn create_dir(dir_path: &str) {
    std::fs::create_dir_all(dir_path).unwrap();
}

fn create_file(file_path: &str, size: usize) {
    let content = vec![0u8; size];
    let file_path = Path::new(file_path);
    // ensure parent
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();

    let mut file = File::create(file_path).unwrap();
    file.write(&content).unwrap();
}

fn assert_size(file_name: &str, apparent: bool, expected_size: u64) {
    if let FileInfo::Directory { volume_id } =
        FileInfo::from_path(&Path::new(TEST_DATA_DIR), apparent).unwrap()
    {
        let result = DiskItem::from_analyze(Path::new(file_name), apparent, volume_id)
            .expect("Shoud be able to get file size");

        assert_eq!(
            expected_size, result.disk_size,
            "Item {:?} size doesn't match expected {}, got {}",
            file_name, expected_size, result.disk_size
        );
    } else {
        panic!("No test-data dir present");
    }
}

#[cfg(windows)]
fn prepare_files_compression() -> std::io::Result<()> {
    use crate::ffi;

    for file in std::fs::read_dir(Path::new(TEST_DATA_DIR))? {
        let file = file?;
        if file.metadata()?.is_dir() {
            continue;
        }

        let file_name = file.file_name();
        let file_name = file_name.as_os_str().to_string_lossy();

        let compress = file_name.ends_with("_c") || file_name.ends_with("_c.txt");

        ffi::set_file_compression(file.path(), compress)?;
    }

    Ok(())
}

/// Used to clean up test folder after test runs.
struct CleanUpGuard {
    path: &'static str,
}

impl Drop for CleanUpGuard {
    fn drop(&mut self) {
        // Teardown
        std::fs::remove_dir_all(Path::new(self.path)).unwrap();
    }
}
