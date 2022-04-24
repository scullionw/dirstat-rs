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

#[test]
fn test_max_path() {
    // do not rename it into `_` it would cause immediate destrucion after creation
    let _guard = CleanUpGuard {
        path: concatcp!(TEST_DATA_DIR, LONG_PATH_DIR) as &str,
    };

    // Given
    create_dir(PATH_1_FULL);
    create_dir(PATH_2_FULL);
    create_file(&concatcp!(PATH_1_FULL, "file.bin") as &str, 4096);
    create_file(&concatcp!(PATH_2_FULL, "file.bin") as &str, 8192);

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
#[cfg(unix)] // It gives inconsistent results on windows
fn test_file_size() {
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
