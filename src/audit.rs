use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use chrono::{DateTime, Local};
use thiserror::Error;
use windows_sys::Win32::Foundation::{CloseHandle, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

use crate::config::{PassCount, SDeleteMode};

const AUDIT_MUTEX_NAME: &str = "Local\\SDeleteGui_AuditLog_Mutex";

/// Errors produced when the audit log cannot be written.
#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Audit log path must not be empty.")]
    EmptyPath,
    #[error("Failed to write audit log: {0}")]
    Io(#[from] std::io::Error),
}

/// Encodes a Rust string into a null-terminated UTF-16 buffer for Win32 APIs.
pub(crate) fn to_wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Formats a single audit record with an ISO-8601 timestamp.
pub fn format_entry(
    timestamp: DateTime<Local>,
    mode: SDeleteMode,
    passes: PassCount,
    target_path: &str,
    exit_code: i32,
    error_detail: Option<&str>,
) -> String {
    let status = if exit_code == 0 { "OK" } else { "ERROR" };
    let iso_time = timestamp.format("%Y-%m-%dT%H:%M:%S%:z");
    let base = format!(
        "{iso_time} | STATUS={status} | MODE={} | PASSES={} | EXIT={exit_code} | TARGET={target_path}",
        mode.as_cli_token(),
        passes.get()
    );

    match error_detail.map(str::trim).filter(|s| !s.is_empty()) {
        Some(detail) => {
            let clean = detail.replace(['\r', '\n'], " ");
            format!("{base} | DETAIL={clean}")
        }
        None => base,
    }
}

/// Appends one formatted audit line under a cross-process Windows mutex.
pub fn append_entry(
    log_path: &Path,
    mode: SDeleteMode,
    passes: PassCount,
    target_path: &str,
    exit_code: i32,
    error_detail: Option<&str>,
) -> Result<(), AuditError> {
    if log_path.as_os_str().is_empty() {
        return Err(AuditError::EmptyPath);
    }

    if let Some(parent) = log_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let line = format_entry(
        Local::now(),
        mode,
        passes,
        target_path,
        exit_code,
        error_detail,
    );

    let wide_name = to_wide_null(AUDIT_MUTEX_NAME);
    // SAFETY: CreateMutexW takes valid pointers and returns a handle that CloseHandle releases.
    let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, wide_name.as_ptr()) };
    let mut locked = false;

    if !mutex.is_null() {
        // SAFETY: mutex is a valid Win32 handle returned by CreateMutexW.
        let wait_result = unsafe { WaitForSingleObject(mutex, 5_000) };
        locked = wait_result == WAIT_OBJECT_0 || wait_result == WAIT_ABANDONED;
    }

    let write_result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        writeln!(file, "{line}")?;
        file.flush()
    })();

    if !mutex.is_null() {
        // SAFETY: ReleaseMutex runs only when locked; CloseHandle frees the valid mutex handle.
        unsafe {
            if locked {
                ReleaseMutex(mutex);
            }
            CloseHandle(mutex);
        }
    }

    write_result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_entry_contains_timestamp_and_fields() {
        let line = format_entry(
            Local::now(),
            SDeleteMode::Delete,
            PassCount::new(3).unwrap(),
            r"C:\Temp\demo.txt",
            0,
            None,
        );
        assert!(line.contains("STATUS=OK"));
        assert!(line.contains("MODE=delete"));
        assert!(line.contains("PASSES=3"));
        assert!(line.contains("EXIT=0"));
        assert!(line.contains(r"TARGET=C:\Temp\demo.txt"));
    }

    #[test]
    fn append_entry_creates_parent_dir_and_sanitizes_newlines() {
        let temp_dir =
            std::env::temp_dir().join(format!("sdelete_gui_audit_{}", std::process::id()));
        let log_file = temp_dir.join("nested").join("audit.log");
        let _ = fs::remove_dir_all(&temp_dir);

        append_entry(
            &log_file,
            SDeleteMode::CleanFreeSpace,
            PassCount::new(1).unwrap(),
            "D:",
            2,
            Some("Access denied\r\nsecond line"),
        )
        .unwrap();

        let content = fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("STATUS=ERROR"));
        assert!(content.contains("DETAIL=Access denied  second line"));
        assert_eq!(content.lines().count(), 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
