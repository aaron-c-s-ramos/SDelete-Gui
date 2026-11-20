use std::path::Path;

use crate::config::{PassCount, SDeleteMode};

pub const RUN_FLAG: &str = "--run";
pub const MODE_FLAG: &str = "--mode";
pub const PASSES_FLAG: &str = "--passes";
pub const TARGET_FLAG: &str = "--target";

/// Parsed command-line request for shell runner execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerRequest {
    pub mode: SDeleteMode,
    pub passes: PassCount,
    pub target_path: String,
}

/// Builds the Explorer registry command string that invokes sdelete-gui in runner mode.
pub fn build_shell_runner_command(
    runner_exe: &Path,
    mode: SDeleteMode,
    passes: PassCount,
) -> String {
    format!(
        "\"{}\" {} {} {} {} {} {} \"%1\"",
        runner_exe.display(),
        RUN_FLAG,
        MODE_FLAG,
        mode.as_cli_token(),
        PASSES_FLAG,
        passes.get(),
        TARGET_FLAG
    )
}

/// Builds the argument vector passed directly to Microsoft Sysinternals sdelete64.exe.
pub fn build_sdelete_args(mode: SDeleteMode, passes: PassCount, raw_target: &str) -> Vec<String> {
    let target = normalize_target_path(raw_target, mode);
    let pass_str = passes.get().to_string();

    match mode {
        SDeleteMode::Delete => vec![
            "-accepteula".to_string(),
            "-p".to_string(),
            pass_str,
            "-s".to_string(),
            "-q".to_string(),
            target,
        ],
        SDeleteMode::CleanFreeSpace => vec![
            "-accepteula".to_string(),
            "-p".to_string(),
            pass_str,
            "-q".to_string(),
            "-c".to_string(),
            target,
        ],
        SDeleteMode::ZeroFreeSpace => vec![
            "-accepteula".to_string(),
            "-p".to_string(),
            pass_str,
            "-q".to_string(),
            "-z".to_string(),
            target,
        ],
    }
}

/// Parses startup CLI arguments into a runner request when `--run` is present.
pub fn parse_runner_args<I, S>(args: I) -> Option<RunnerRequest>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let items: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if items.is_empty() {
        return None;
    }

    let mut has_run = false;
    let mut mode = SDeleteMode::Delete;
    let mut passes = PassCount::default();
    let mut target: Option<String> = None;

    let mut idx = 0;
    while idx < items.len() {
        let arg = &items[idx];
        if arg.eq_ignore_ascii_case(RUN_FLAG) {
            has_run = true;
        } else if arg.eq_ignore_ascii_case(MODE_FLAG) && idx + 1 < items.len() {
            idx += 1;
            mode = SDeleteMode::parse_cli_token(&items[idx])?;
        } else if arg.eq_ignore_ascii_case(PASSES_FLAG) && idx + 1 < items.len() {
            idx += 1;
            let raw: u8 = items[idx].parse().ok()?;
            passes = PassCount::new(raw).ok()?;
        } else if arg.eq_ignore_ascii_case(TARGET_FLAG) && idx + 1 < items.len() {
            idx += 1;
            target = Some(items[idx].clone());
        }
        idx += 1;
    }

    if !has_run {
        return None;
    }

    let raw_target = target?;
    let normalized = normalize_target_path(&raw_target, mode);
    if normalized.is_empty() {
        return None;
    }

    Some(RunnerRequest {
        mode,
        passes,
        target_path: normalized,
    })
}

/// Normalizes drive roots and strips stray quotes caused by Explorer backslash escaping.
pub fn normalize_target_path(raw: &str, mode: SDeleteMode) -> String {
    let cleaned = raw.trim().trim_matches('"').trim();
    if matches!(
        mode,
        SDeleteMode::CleanFreeSpace | SDeleteMode::ZeroFreeSpace
    ) {
        let bytes = cleaned.as_bytes();
        if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return cleaned[..2].to_string();
        }
    }
    cleaned.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_and_zero_free_space_flags_stay_separate() {
        let p = PassCount::new(3).unwrap();
        let clean = build_sdelete_args(SDeleteMode::CleanFreeSpace, p, r"D:\");
        assert_eq!(clean, vec!["-accepteula", "-p", "3", "-q", "-c", "D:"]);
        assert!(!clean.contains(&"-z".to_string()));

        let zero = build_sdelete_args(SDeleteMode::ZeroFreeSpace, p, r"C:\");
        assert_eq!(zero, vec!["-accepteula", "-p", "3", "-q", "-z", "C:"]);
        assert!(!zero.contains(&"-c".to_string()));
    }

    #[test]
    fn delete_mode_preserves_spaces_in_target() {
        let p = PassCount::new(7).unwrap();
        let args = build_sdelete_args(SDeleteMode::Delete, p, r"C:\Temp\my file.txt");
        assert_eq!(
            args,
            vec!["-accepteula", "-p", "7", "-s", "-q", r"C:\Temp\my file.txt"]
        );
    }

    #[test]
    fn parse_runner_args_handles_explorer_drive_escaping() {
        let req = parse_runner_args([
            "--run",
            "--mode",
            "clean-free-space",
            "--passes",
            "5",
            "--target",
            "D:\\\"",
        ])
        .unwrap();
        assert_eq!(req.mode, SDeleteMode::CleanFreeSpace);
        assert_eq!(req.passes.get(), 5);
        assert_eq!(req.target_path, "D:");
    }

    #[test]
    fn parse_runner_args_rejects_invalid_pass_count() {
        assert!(parse_runner_args(["--run", "--passes", "0", "--target", "file.txt"]).is_none());
        assert!(parse_runner_args(["--run", "--passes", "101", "--target", "file.txt"]).is_none());
        assert!(parse_runner_args(["--passes", "3", "--target", "file.txt"]).is_none());
    }
}
