//! 练习资产路径映射回归测试。

use super::roots::bundle_candidates_from_resource_dir;
use std::path::PathBuf;

#[test]
fn bundle_candidates_cover_book_and_course_subdirs() {
    let resource_dir = PathBuf::from("/Application/App.app/Contents/Resources");
    let candidates = bundle_candidates_from_resource_dir(&resource_dir);

    assert!(candidates.contains(&resource_dir.join("books")));
    assert!(candidates.contains(&resource_dir.join("courses")));
    assert!(candidates.contains(&resource_dir.join("assets")));
    assert!(candidates.contains(&resource_dir.join("assets").join("books")));
    assert!(candidates.contains(&resource_dir.join("assets").join("courses")));
}
