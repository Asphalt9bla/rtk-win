use super::constants::NOISE_DIRS;
use crate::core::guard::never_worse;
use crate::core::tracking;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const DEFAULT_MAX_LINES: usize = 80;

fn truncate_lines(s: &str, max: usize) -> String {
    let total: Vec<&str> = s.lines().collect();
    if total.len() <= max {
        return s.to_string();
    }
    let omit = total.len() - max;
    let mut out: String = total[..max].join("\n");
    out.push_str(&format!("\n... ({} lines truncated)", omit));
    out
}

pub fn run(args: &[String], verbose: u8) -> Result<i32> {
    let timer = tracking::TimedExecution::start();

    let show_all = args.iter().any(|a| a == "-a" || a == "--all");
    let target = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(|s| s.as_str())
        .unwrap_or(".");

    if verbose > 0 {
        eprintln!("Tree: {} (show_all: {})", target, show_all);
    }

    let output = build_tree(target, show_all)
        .with_context(|| format!("Failed to build tree for: {}", target))?;

    let trunk = truncate_lines(&output, DEFAULT_MAX_LINES);
    let shown = never_worse(&output, &trunk);
    print!("{}", shown);

    if verbose > 0 {
        eprintln!(
            "Chars: {} → {} ({}% reduction)",
            output.len(),
            shown.len(),
            if !output.is_empty() {
                100 - (shown.len() * 100 / output.len())
            } else {
                0
            }
        );
    }

    timer.track(
        &format!("tree {}", target),
        "rtk tree",
        &output,
        shown,
    );
    Ok(0)
}

fn build_tree(root: &str, show_all: bool) -> Result<String> {
    let root_path = Path::new(root);
    if !root_path.exists() {
        anyhow::bail!("Directory not found: {}", root);
    }

    let root_name = root_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string());

    let entries: Vec<PathBuf> = WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| {
            if show_all {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            if name.starts_with('.') {
                return false;
            }
            !NOISE_DIRS.iter().any(|noise| name == *noise)
        })
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .collect();

    let mut children_map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for entry in &entries {
        if let Some(parent) = entry.parent() {
            children_map.entry(parent.to_path_buf()).or_default().push(entry.clone());
        }
    }

    for list in children_map.values_mut() {
        list.sort_by(|a, b| {
            let a_is_dir = a.is_dir();
            let b_is_dir = b.is_dir();
            if a_is_dir != b_is_dir {
                b_is_dir.cmp(&a_is_dir)
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });
    }

    let mut result = String::new();
    result.push_str(&root_name);
    result.push('\n');

    let root_children = children_map.remove(root_path).unwrap_or_default();
    for (i, child) in root_children.iter().enumerate() {
        let is_last = i == root_children.len() - 1;
        let name = child.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        print_entry(&mut result, "", child, &name, is_last, &children_map);
    }

    Ok(result)
}

fn print_entry(
    result: &mut String,
    prefix: &str,
    path: &Path,
    name: &str,
    is_last: bool,
    children_map: &HashMap<PathBuf, Vec<PathBuf>>,
) {
    let connector = if is_last { "└── " } else { "├── " };
    let suffix = if path.is_dir() { "/" } else { "" };

    result.push_str(&format!("{}{}{}{}\n", prefix, connector, name, suffix));

    if path.is_dir() {
        let child_prefix = if is_last { "    " } else { "│   " };
        let children = children_map.get(path).map(|v| v.as_slice()).unwrap_or(&[]);
        for (i, child) in children.iter().enumerate() {
            let child_is_last = i == children.len() - 1;
            let child_name = child.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let full_prefix = format!("{}{}", prefix, child_prefix);
            print_entry(result, &full_prefix, child, &child_name, child_is_last, children_map);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]\n").unwrap();
        fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(temp.path().join("src/lib.rs"), "pub fn foo() {}\n").unwrap();
        temp
    }

    #[test]
    fn test_tree_contains_files() {
        let temp = setup_test_dir();
        let output = build_tree(temp.path().to_str().unwrap(), false).unwrap();
        assert!(output.contains("Cargo.toml"));
        assert!(output.contains("src/"));
        assert!(output.contains("main.rs"));
        assert!(output.contains("lib.rs"));
    }

    #[test]
    fn test_tree_root_is_dir_name() {
        let temp = setup_test_dir();
        let dir_name = temp.path().file_name().unwrap().to_string_lossy().to_string();
        let output = build_tree(temp.path().to_str().unwrap(), false).unwrap();
        assert!(output.starts_with(&dir_name));
    }

    #[test]
    fn test_tree_uses_tree_chars() {
        let temp = setup_test_dir();
        let output = build_tree(temp.path().to_str().unwrap(), false).unwrap();
        assert!(output.contains("├──") || output.contains("└──"));
    }

    #[test]
    fn test_noise_dirs_constant() {
        assert!(NOISE_DIRS.contains(&"node_modules"));
        assert!(NOISE_DIRS.contains(&".git"));
        assert!(NOISE_DIRS.contains(&"target"));
        assert!(NOISE_DIRS.contains(&"__pycache__"));
        assert!(NOISE_DIRS.contains(&".next"));
        assert!(NOISE_DIRS.contains(&"dist"));
        assert!(NOISE_DIRS.contains(&"build"));
    }

    #[test]
    fn test_tree_filters_noise() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("node_modules")).unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("main.rs"), "fn main() {}\n").unwrap();

        let output = build_tree(temp.path().to_str().unwrap(), false).unwrap();
        assert!(!output.contains("node_modules"));
        assert!(output.contains("src/"));
        assert!(output.contains("main.rs"));
    }

    #[test]
    fn test_tree_show_all_includes_hidden() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();
        fs::write(temp.path().join(".hidden"), "data").unwrap();

        let output = build_tree(temp.path().to_str().unwrap(), true).unwrap();
        assert!(output.contains(".git/"));
        assert!(output.contains(".hidden"));
    }
}
