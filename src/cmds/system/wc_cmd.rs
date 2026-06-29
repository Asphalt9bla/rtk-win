use crate::core::guard::never_worse;
use crate::core::tracking;
use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, PartialEq)]
enum WcMode {
    Full,
    Lines,
    Words,
    Bytes,
    Chars,
    Mixed,
}

pub fn run(args: &[String], verbose: u8) -> Result<i32> {
    let timer = tracking::TimedExecution::start();

    let mode = detect_mode(args);
    let file_args: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(|s| s.as_str())
        .collect();

    let output = if file_args.is_empty() {
        count_stdin(&mode)?
    } else {
        count_files(&file_args, &mode)?
    };

    let shown = never_worse(&output, &output);
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
        &format!("wc {}", args.join(" ")),
        "rtk wc",
        &output,
        shown,
    );
    Ok(0)
}

fn detect_mode(args: &[String]) -> WcMode {
    let flags: Vec<&str> = args
        .iter()
        .filter(|a| a.starts_with('-'))
        .map(|s| s.as_str())
        .collect();

    if flags.is_empty() {
        return WcMode::Full;
    }

    let mut has_l = false;
    let mut has_w = false;
    let mut has_c = false;
    let mut has_m = false;
    let mut flag_count = 0;

    for flag in &flags {
        for ch in flag.chars().skip(1) {
            match ch {
                'l' => { has_l = true; flag_count += 1; }
                'w' => { has_w = true; flag_count += 1; }
                'c' => { has_c = true; flag_count += 1; }
                'm' => { has_m = true; flag_count += 1; }
                _ => {}
            }
        }
    }

    if flag_count == 0 {
        return WcMode::Full;
    }
    if flag_count > 1 {
        return WcMode::Mixed;
    }

    if has_l { WcMode::Lines }
    else if has_w { WcMode::Words }
    else if has_c { WcMode::Bytes }
    else if has_m { WcMode::Chars }
    else { WcMode::Full }
}

struct Counts {
    lines: usize,
    words: usize,
    bytes: usize,
    chars: usize,
}

fn count_content(content: &str) -> Counts {
    let lines = content.bytes().filter(|&b| b == b'\n').count();
    let words = content.split_whitespace().count();
    let bytes = content.len();
    let chars = content.chars().count();
    Counts { lines, words, bytes, chars }
}

fn count_stdin(mode: &WcMode) -> Result<String> {
    let mut content = String::new();
    io::stdin().read_to_string(&mut content)?;
    let counts = count_content(&content);
    Ok(format_counts_single(&counts, mode))
}

fn count_files(files: &[&str], mode: &WcMode) -> Result<String> {
    if files.len() == 1 {
        let content = fs::read_to_string(files[0])
            .with_context(|| format!("Failed to read file: {}", files[0]))?;
        let counts = count_content(&content);
        return Ok(format_counts_single(&counts, mode));
    }

    let mut all_counts: Vec<(String, Counts)> = Vec::new();
    let mut total = Counts { lines: 0, words: 0, bytes: 0, chars: 0 };

    let common_prefix = find_common_prefix(files);

    for file in files {
        let path = Path::new(file);
        match fs::read_to_string(path) {
            Ok(content) => {
                let counts = count_content(&content);
                let display_name = file.strip_prefix(&common_prefix).unwrap_or(file);
                all_counts.push((display_name.to_string(), Counts { ..counts }));
                total.lines += counts.lines;
                total.words += counts.words;
                total.bytes += counts.bytes;
                total.chars += counts.chars;
            }
            Err(e) => {
                eprintln!("wc: {}: {}", file, e);
            }
        }
    }

    Ok(format_counts_multi(&all_counts, &total, mode))
}

fn format_counts_single(counts: &Counts, mode: &WcMode) -> String {
    match mode {
        WcMode::Lines => counts.lines.to_string(),
        WcMode::Words => counts.words.to_string(),
        WcMode::Bytes => counts.bytes.to_string(),
        WcMode::Chars => counts.chars.to_string(),
        WcMode::Full => format!("{}L {}W {}B", counts.lines, counts.words, counts.bytes),
        WcMode::Mixed => {
            [counts.lines.to_string(), counts.words.to_string(), counts.bytes.to_string()].join(" ")
        }
    }
}

fn format_counts_multi(
    all_counts: &[(String, Counts)],
    total: &Counts,
    mode: &WcMode,
) -> String {
    let mut result = Vec::new();

    for (name, counts) in all_counts {
        let line = match mode {
            WcMode::Lines => format!("{} {}", counts.lines, name),
            WcMode::Words => format!("{} {}", counts.words, name),
            WcMode::Bytes => format!("{} {}", counts.bytes, name),
            WcMode::Chars => format!("{} {}", counts.chars, name),
            WcMode::Full => format!("{}L {}W {}B {}", counts.lines, counts.words, counts.bytes, name),
            WcMode::Mixed => {
                let nums = [counts.lines.to_string(), counts.words.to_string(), counts.bytes.to_string()];
                format!("{} {}", nums.join(" "), name)
            }
        };
        result.push(line);
    }

    let total_line = match mode {
        WcMode::Lines => format!("Σ {}", total.lines),
        WcMode::Words => format!("Σ {}", total.words),
        WcMode::Bytes => format!("Σ {}", total.bytes),
        WcMode::Chars => format!("Σ {}", total.chars),
        WcMode::Full => format!("Σ {}L {}W {}B", total.lines, total.words, total.bytes),
        WcMode::Mixed => {
            let nums = [total.lines.to_string(), total.words.to_string(), total.bytes.to_string()];
            format!("Σ {}", nums.join(" "))
        }
    };
    result.push(total_line);

    result.join("\n")
}

fn find_common_prefix(paths: &[&str]) -> String {
    if paths.len() <= 1 {
        return String::new();
    }

    let first = paths[0];
    let prefix = if let Some(pos) = first.rfind(['/', '\\']) {
        &first[..=pos]
    } else {
        return String::new();
    };

    if paths.iter().all(|p| p.starts_with(prefix)) {
        return prefix.to_string();
    }

    let mut candidate = prefix.to_string();
    while !candidate.is_empty() {
        if paths.iter().all(|p| p.starts_with(&candidate)) {
            return candidate;
        }
        if let Some(pos) = candidate[..candidate.len() - 1].rfind(['/', '\\']) {
            candidate.truncate(pos + 1);
        } else {
            return String::new();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_mode_full() {
        let args: Vec<String> = vec!["file.py".into()];
        assert_eq!(detect_mode(&args), WcMode::Full);
    }

    #[test]
    fn test_detect_mode_lines() {
        let args: Vec<String> = vec!["-l".into(), "file.py".into()];
        assert_eq!(detect_mode(&args), WcMode::Lines);
    }

    #[test]
    fn test_detect_mode_mixed() {
        let args: Vec<String> = vec!["-lw".into(), "file.py".into()];
        assert_eq!(detect_mode(&args), WcMode::Mixed);
    }

    #[test]
    fn test_detect_mode_separate_flags() {
        let args: Vec<String> = vec!["-l".into(), "-w".into(), "file.py".into()];
        assert_eq!(detect_mode(&args), WcMode::Mixed);
    }

    #[test]
    fn test_format_counts_single_full() {
        let counts = Counts { lines: 30, words: 96, bytes: 978, chars: 978 };
        assert_eq!(format_counts_single(&counts, &WcMode::Full), "30L 96W 978B");
    }

    #[test]
    fn test_format_counts_single_lines() {
        let counts = Counts { lines: 30, words: 96, bytes: 978, chars: 978 };
        assert_eq!(format_counts_single(&counts, &WcMode::Lines), "30");
    }

    #[test]
    fn test_format_counts_single_words() {
        let counts = Counts { lines: 30, words: 96, bytes: 978, chars: 978 };
        assert_eq!(format_counts_single(&counts, &WcMode::Words), "96");
    }

    #[test]
    fn test_format_counts_multi_lines() {
        let counts = vec![
            ("src/main.rs".to_string(), Counts { lines: 30, words: 96, bytes: 978, chars: 978 }),
            ("src/lib.rs".to_string(), Counts { lines: 50, words: 120, bytes: 1500, chars: 1500 }),
        ];
        let total = Counts { lines: 80, words: 216, bytes: 2478, chars: 2478 };
        let result = format_counts_multi(&counts, &total, &WcMode::Lines);
        assert_eq!(result, "30 src/main.rs\n50 src/lib.rs\nΣ 80");
    }

    #[test]
    fn test_format_counts_multi_full() {
        let counts = vec![
            ("main.rs".to_string(), Counts { lines: 30, words: 96, bytes: 978, chars: 978 }),
            ("lib.rs".to_string(), Counts { lines: 50, words: 120, bytes: 1500, chars: 1500 }),
        ];
        let total = Counts { lines: 80, words: 216, bytes: 2478, chars: 2478 };
        let result = format_counts_multi(&counts, &total, &WcMode::Full);
        assert_eq!(result, "30L 96W 978B main.rs\n50L 120W 1500B lib.rs\nΣ 80L 216W 2478B");
    }

    #[test]
    fn test_common_prefix_unix() {
        let paths = vec!["src/main.rs", "src/lib.rs", "src/utils.rs"];
        assert_eq!(find_common_prefix(&paths), "src/");
    }

    #[test]
    fn test_common_prefix_windows() {
        let paths = vec!["src\\main.rs", "src\\lib.rs"];
        assert_eq!(find_common_prefix(&paths), "src\\");
    }

    #[test]
    fn test_no_common_prefix() {
        let paths = vec!["main.rs", "lib.rs"];
        assert_eq!(find_common_prefix(&paths), "");
    }

    #[test]
    fn test_common_prefix_with_prefix_stripping() {
        let paths = vec!["src/main.rs", "src/lib.rs"];
        let prefix = find_common_prefix(&paths);
        let stripped: Vec<&str> = paths.iter().map(|p| p.strip_prefix(&prefix).unwrap_or(p)).collect();
        assert_eq!(stripped, vec!["main.rs", "lib.rs"]);
    }

    #[test]
    fn test_count_content_simple() {
        let counts = count_content("hello world\nfoo bar baz\n");
        assert_eq!(counts.lines, 2);
        assert_eq!(counts.words, 5);
        assert_eq!(counts.bytes, 24);
    }

    #[test]
    fn test_count_content_empty() {
        let counts = count_content("");
        assert_eq!(counts.lines, 0);
        assert_eq!(counts.words, 0);
        assert_eq!(counts.bytes, 0);
    }

    #[test]
    fn test_count_content_single_line_with_newline() {
        let counts = count_content("hello\n");
        assert_eq!(counts.lines, 1);
        assert_eq!(counts.words, 1);
        assert_eq!(counts.bytes, 6);
    }
}
