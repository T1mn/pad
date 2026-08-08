mod build {
    use super::super::build::build_tree;
    use crate::test_support;
    use std::collections::HashSet;
    use std::fs;

    #[test]
    fn build_tree_skips_ignored_directories_before_rows() {
        let root =
            test_support::temp_path("pad_sider_tree", "build_tree_skips_ignored_directories");
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("target/hidden")).unwrap();
        fs::write(root.join("docs/readme.md"), "# ok").unwrap();
        fs::write(root.join("target/hidden/readme.md"), "# hidden").unwrap();

        let rows = build_tree(&root, &HashSet::new());

        assert!(rows.iter().any(|row| row.path.ends_with("docs")));
        assert!(!rows.iter().any(|row| row.path.ends_with("target")));
        assert!(!rows
            .iter()
            .any(|row| row.path.ends_with("target/hidden/readme.md")));

        fs::remove_dir_all(root).unwrap();
    }
}

mod scan {
    use super::super::scan::scan_files;
    use crate::test_support;
    use std::fs;

    #[test]
    fn scan_files_skips_ignored_directories() {
        let root = temp_dir("scan_files_skips_ignored_directories");
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("docs/readme.md"), "# ok").unwrap();
        fs::write(root.join(".git/config"), "ignored").unwrap();

        let files = scan_files(&root);
        assert!(files.iter().any(|path| path.ends_with("docs/readme.md")));
        assert!(!files.iter().any(|path| path.ends_with(".git/config")));

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        test_support::temp_path("pad_sider", name)
    }
}
