use super::constants::NOISE_DIRS;
use crate::core::guard::never_worse;
use crate::core::tracking;
use crate::core::truncate::{reduced, CAP_WARNINGS};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::IsTerminal;

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

    let show_all = args
        .iter()
        .any(|a| (a.starts_with('-') && !a.starts_with("--") && a.contains('a')) || a == "--all");
    let show_long = args.iter().any(|a| {
        if a == "--full-time" || a == "--format=long" || a == "--format=verbose" {
            return true;
        }
        if a.starts_with('-') && !a.starts_with("--") {
            return a.chars().any(|c| matches!(c, 'l' | 'g' | 'n' | 'o'));
        }
        false
    });

    let paths: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(|s| s.as_str())
        .collect();
    let target = if paths.is_empty() { "." } else { paths[0] };

    if verbose > 0 {
        eprintln!("Listing: {} (filter: {})", target, if show_long { "long" } else { "short" });
    }

    let (entries, summary) = compact_dir(target, show_all, show_long)
        .with_context(|| format!("Failed to list directory: {}", target))?;

    let is_tty = std::io::stdout().is_terminal();
    let output = if is_tty {
        format!("{}{}", entries, summary)
    } else {
        entries
    };

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
        &format!("ls {}", target),
        "rtk ls",
        &output,
        shown,
    );
    Ok(0)
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1}M", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}

fn get_file_type_char(metadata: &fs::Metadata) -> char {
    if metadata.is_dir() {
        'd'
    } else if metadata.file_type().is_symlink() {
        'l'
    } else {
        '-'
    }
}

fn get_perms_octal(metadata: &fs::Metadata) -> Option<String> {
    let perms = metadata.permissions();
    if metadata.is_dir() {
        Some("755".to_string())
    } else if perms.readonly() {
        Some("444".to_string())
    } else {
        Some("644".to_string())
    }
}

fn compact_dir(target: &str, show_all: bool, show_long: bool) -> Result<(String, String)> {
    let mut dirs: Vec<(String, Option<String>)> = Vec::new();
    let mut files: Vec<(String, String, Option<String>)> = Vec::new();
    let mut by_ext: HashMap<String, usize> = HashMap::new();

    let read_dir = fs::read_dir(target)
        .with_context(|| format!("Failed to read directory: {}", target))?;

    for entry in read_dir {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        if !show_all && name.starts_with('.') {
            continue;
        }

        if !show_all && NOISE_DIRS.iter().any(|noise| name == *noise) {
            continue;
        }

        let metadata = fs::symlink_metadata(entry.path())
            .with_context(|| format!("Failed to get metadata for: {}", name))?;

        let ft = get_file_type_char(&metadata);
        let octal = if show_long { get_perms_octal(&metadata) } else { None };

        if ft == 'd' {
            dirs.push((name, octal));
        } else {
            let ext = if let Some(pos) = name.rfind('.') {
                name[pos..].to_string()
            } else {
                "no ext".to_string()
            };
            *by_ext.entry(ext).or_insert(0) += 1;
            let size = metadata.len();
            files.push((name, human_size(size), octal));
        }
    }

    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    if dirs.is_empty() && files.is_empty() {
        return Ok(("(empty)\n".to_string(), String::new()));
    }

    let mut entries = String::new();

    for (name, octal) in &dirs {
        if let Some(octal) = octal {
            entries.push_str(octal);
            entries.push_str("  ");
        }
        entries.push_str(name);
        entries.push_str("/\n");
    }

    for (name, size, octal) in &files {
        if let Some(octal) = octal {
            entries.push_str(octal);
            entries.push_str("  ");
        }
        entries.push_str(name);
        entries.push_str("  ");
        entries.push_str(size);
        entries.push('\n');
    }

    let mut summary = format!("\nSummary: {} files, {} dirs", files.len(), dirs.len());
    if !by_ext.is_empty() {
        const MAX_EXT_SUMMARY: usize = reduced(CAP_WARNINGS, 5);
        let mut ext_counts: Vec<_> = by_ext.iter().collect();
        ext_counts.sort_by(|a, b| b.1.cmp(a.1));
        let ext_parts: Vec<String> = ext_counts
            .iter()
            .take(MAX_EXT_SUMMARY)
            .map(|(ext, count)| format!("{} {}", count, ext))
            .collect();
        summary.push_str(" (");
        summary.push_str(&ext_parts.join(", "));
        if ext_counts.len() > MAX_EXT_SUMMARY {
            summary.push_str(&format!(", +{} more", ext_counts.len() - MAX_EXT_SUMMARY));
        }
        summary.push(')');
    }
    summary.push('\n');

    Ok((entries, summary))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compact_basic() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("Cargo.toml"), "a".repeat(1234)).unwrap();
        fs::write(temp.path().join("README.md"), "b".repeat(5678)).unwrap();

        let (entries, _summary) = compact_dir(temp.path().to_str().unwrap(), false, false).unwrap();
        assert!(entries.contains("src/"));
        assert!(entries.contains("Cargo.toml"));
        assert!(entries.contains("README.md"));
        assert!(entries.contains("1.2K"));
        assert!(entries.contains("5.5K"));
    }

    #[test]
    fn test_compact_filters_noise() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("node_modules")).unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();
        fs::create_dir(temp.path().join("target")).unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("main.rs"), "fn main() {}").unwrap();

        let (entries, _summary) = compact_dir(temp.path().to_str().unwrap(), false, false).unwrap();
        assert!(!entries.contains("node_modules"));
        assert!(!entries.contains(".git"));
        assert!(!entries.contains("target"));
        assert!(entries.contains("src/"));
        assert!(entries.contains("main.rs"));
    }

    #[test]
    fn test_compact_show_all() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join(".hidden"), "secret").unwrap();

        let (entries, _summary) = compact_dir(temp.path().to_str().unwrap(), true, false).unwrap();
        assert!(entries.contains(".git/"));
        assert!(entries.contains("src/"));
        assert!(entries.contains(".hidden"));
    }

    #[test]
    fn test_compact_empty() {
        let temp = TempDir::new().unwrap();
        let (entries, _summary) = compact_dir(temp.path().to_str().unwrap(), false, false).unwrap();
        assert_eq!(entries, "(empty)\n");
    }

    #[test]
    fn test_human_size() {
        assert_eq!(human_size(0), "0B");
        assert_eq!(human_size(500), "500B");
        assert_eq!(human_size(1024), "1.0K");
        assert_eq!(human_size(1234), "1.2K");
        assert_eq!(human_size(1_048_576), "1.0M");
        assert_eq!(human_size(2_500_000), "2.4M");
    }

    #[test]
    fn test_compact_long_format_includes_octal() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]").unwrap();

        let (entries, _summary) = compact_dir(temp.path().to_str().unwrap(), false, true).unwrap();
        assert!(entries.contains("755  src/"));
        assert!(entries.contains("644  Cargo.toml"));
    }

    #[test]
    fn test_compact_summary() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(temp.path().join("lib.rs"), "pub fn foo() {}").unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]\n").unwrap();

        let (_entries, summary) = compact_dir(temp.path().to_str().unwrap(), false, false).unwrap();
        assert!(summary.contains("Summary: 3 files, 1 dirs"));
        assert!(summary.contains(".rs"));
        assert!(summary.contains(".toml"));
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
}
