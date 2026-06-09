/// Scan directories up to max_depth (public for use by App)
pub fn scan_directories(base: &str, max_depth: usize) -> Vec<String> {
    let mut results = vec![base.to_string()];

    if max_depth == 0 {
        return results;
    }

    let base_path = std::path::Path::new(base);
    if let Ok(entries) = std::fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let path_str = path.to_string_lossy().to_string();

                // Skip hidden directories
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        continue;
                    }
                }

                // Recursively scan (limit depth)
                if max_depth > 1 {
                    let sub_dirs = scan_directories(&path_str, max_depth - 1);
                    results.extend(sub_dirs.into_iter().skip(1)); // Skip duplicate base
                }
                results.push(path_str);
            }
        }
    }

    // Sort and remove duplicates
    results.sort_unstable();
    results.dedup();
    results
}
