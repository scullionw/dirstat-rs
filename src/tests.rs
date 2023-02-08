//! Be aware that tests are run in parallel. For this reason we must be sure to use
//! separate dirs for different cases.
#![allow(dead_code)]

use crate::{DiskItem, FileInfo};
use const_format::concatcp;
use rstest::*;
use std::fs::File;
use std::io::Write;
use std::panic;
use std::path::Path;

/// Base directory for files used in tests.
///
/// Be aware that rust runs tests in parallel, so tests should use different dirs.
const TEST_DATA_DIR: &str = "./test-data/";

/// Path for test case for very long path.
const LONG_PATH_DIR: &str = "long-path/";

//noinspection SpellCheckingInspection
const PATH_1: &str = "lll1/llllllll/llllllllllllllll/llllllllllllll/lllllllllllll/oooooo\
oooooooo/oooooooooooooooo/nnnnnnnnn/nnnnnnnnnn/nnnnnnnn/nnnnnn/gggggggggg/p/a/tttt\
tttttttttt/ttttttttttt/ttttttttttttttt/ttttttttt/tttthhh/2222222222/22222222222/222222222222/\
3333333333333/33333333/33333333333/33333333333/333333333/33333333/44444444/44444444444444444/\
5555555/55555555555/55555555/5555555555/5555555/5555555/555555/555555555/66666666666666666666/\
77777777/7777777777/7777777777777/77777777777/7777777777/77777777/7777777/77777777/8888888888/\
99999999/999999/99999999/99999999999/99999999/999999999/9999999999/";

const PATH_1_FULL: &str = concatcp!(TEST_DATA_DIR, LONG_PATH_DIR, PATH_1);
//noinspection SpellCheckingInspection
const PATH_2: &str = "lll2/llllllll/llllllllllllllll/llllllllllllll/lllllllllllll/oooooo\
oooooooo/oooooooooooooooo/nnnnnnnnn/nnnnnnnnnn/nnnnnnnn/nnnnnn/gggggggggg/p/a/tttt\
tttttttttt/ttttttttttt/ttttttttttttttt/ttttttttt/tttthhh/2222222222/22222222222/222222222222/\
3333333333333/33333333/33333333333/33333333333/333333333/33333333/44444444/44444444444444444/\
5555555/55555555555/55555555/5555555555/5555555/5555555/555555/555555555/66666666666666666666/\
77777777/7777777777/7777777777777/77777777777/7777777777/77777777/7777777/77777777/8888888888/\
99999999/999999/99999999/99999999999/99999999/999999999/9999999999/";

const PATH_2_FULL: &str = concatcp!(TEST_DATA_DIR, LONG_PATH_DIR, PATH_2);

const TEST_PRE_CREATED_DIR: &str = concatcp!(TEST_DATA_DIR, "pre-created/");

#[test]
#[cfg(not(windows))]
fn test_max_path() {
    // do not rename it into `_` it would cause immediate destruction after creation
    let _guard = CleanUpGuard {
        path: concatcp!(TEST_DATA_DIR, LONG_PATH_DIR),
    };

    // Given
    create_dir(PATH_1_FULL);
    create_dir(PATH_2_FULL);
    create_file(&concatcp!(PATH_1_FULL, "file.bin"), 4096);
    create_file(&concatcp!(PATH_2_FULL, "file.bin"), 8192);

    // When
    let test_path = Path::new(concatcp!(TEST_DATA_DIR, LONG_PATH_DIR));
    let result = FileInfo::from_path(test_path, true);

    // Then
    if let Ok(FileInfo::Directory { volume_id }) = result {
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

#[rstest]
#[case("", 53782)]
#[case("b23_rand", 23)]
#[case("b23_zero", 23)]
#[case("b4000_rand", 4000)]
#[case("b4000_zero", 4000)]
#[case("b4096_rand", 4096)]
#[case("b4096_zero", 4096)]
#[case("b512_rand", 512)]
#[case("b512_zero", 512)]
#[case("b8000_rand", 8000)]
#[case("b8192_rand", 8192)]
#[case("b8192_zero", 8192)]
#[case("rand_1000", 1000)]
#[case("text1.txt", 2088)]
#[case("text2.txt", 9048)]
fn test_files_logical_size(#[case] file: &str, #[case] size: u64) {
    let file = String::from(TEST_PRE_CREATED_DIR) + file;
    assert_size(&file, false, size);
}

#[rstest]
// Can't test top dir, as compressed files would mess the picture so no test for ""
#[cfg_attr(windows, case("b23_rand", 23))]
#[cfg_attr(windows, case("b23_zero", 23))]
#[cfg_attr(windows, case("b512_rand", 512))]
#[cfg_attr(windows, case("b512_zero", 512))]
// TODO this is really FS dependant. On WSL and ntfs it all would be 0. With Ext4 it would be 4096
// either add FS specific logic, or don't assert this. I guss second option, as otherwise tests
// aren't reproducible.
// #[cfg_attr(not(windows),case("b23_rand", 0))]
// #[cfg_attr(not(windows),case("b23_zero", 0))]
// #[cfg_attr(not(windows),case("b512_rand", 0))]
// #[cfg_attr(not(windows),case("b512_zero", 0))]
#[cfg_attr(not(windows), case("b4000_rand", 4096))]
#[cfg_attr(not(windows), case("b4000_zero", 4096))]
#[case("b4096_rand", 4096)]
#[case("b4096_zero", 4096)]
#[cfg_attr(not(windows), case("b8000_rand", 8192))]
#[case("b8192_rand", 8192)]
#[case("b8192_zero", 8192)]
#[cfg_attr(not(windows), case("rand_1000", 4096))]
#[cfg_attr(not(windows), case("text1.txt", 4096))]
#[cfg_attr(not(windows), case("text2.txt", 12288))]
fn test_files_physical_size(#[case] file: &str, #[case] size: u64) {
    let file = String::from(TEST_PRE_CREATED_DIR) + file;
    assert_size(&file, true, size);
}

#[allow(non_snake_case)]
#[test]
fn test_file_size_8KiB() {
    const DIR: &str = concatcp!(TEST_DATA_DIR, "test_file_size/");
    // do not rename it into `_` it would cause immediate destruction after creation
    let _guard = CleanUpGuard { path: DIR };

    // Given
    // Such sizes is selected to be close to filesystem sector size, and to be maximally universal
    // event for FS-es with sector as bif as 8KiB
    create_file(&concatcp!(DIR, "foo/file.bin"), 8192);
    create_file(&concatcp!(DIR, "bar/file.bin"), 8192 - 5);

    // When calculating with apparent size
    let test_path = Path::new(DIR);
    let result = FileInfo::from_path(test_path, true);

    // Then
    if let Result::Ok(FileInfo::Directory { volume_id }) = result {
        let result = DiskItem::from_analyze(test_path, true, volume_id);
        let result = result.expect("Must collect data");
        if cfg!(not(windows)) {
            // TODO this check fails fon windows currently.
            assert_eq!(result.disk_size, 8192 + 8192);
        }
        let children = result.children.unwrap();
        assert_eq!(children.len(), 2);
        // Both dirs should be rounded to sector size
        assert_eq!(children[0].disk_size, 8192);
        if cfg!(not(windows)) {
            // TODO this check fails fon windows currently.
            assert_eq!(children[1].disk_size, 8192);
        }
    } else {
        panic!("Can not get file info");
    }

    // When calculating without apparent size
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
            .expect("Should be able to get file size");

        assert_eq!(
            expected_size, result.disk_size,
            "Item {:?} size doesn't match expected {}, got {}",
            file_name, expected_size, result.disk_size
        );
    } else {
        panic!("No test-data dir present");
    }
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
