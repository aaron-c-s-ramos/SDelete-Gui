use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, ERROR_IO_PENDING, ERROR_PIPE_CONNECTED,
    HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
};
use windows_sys::Win32::Storage::FileSystem::{
    ReadFile, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_INBOUND,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, ReleaseMutex, WaitForSingleObject,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONERROR, MB_ICONWARNING, MB_OK, MB_TOPMOST, MB_YESNO,
};

use crate::audit::{append_entry, to_wide_null};
use crate::command::{build_sdelete_args, RunnerRequest};
use crate::config::{SDeleteConfig, SDeleteMode};
use crate::i18n::strings_for;
use crate::sdelete_bin::find_existing_sdelete;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const BATCH_DEBOUNCE: Duration = Duration::from_millis(350);
const MAX_BATCH_WAIT: Duration = Duration::from_millis(2500);
const MAX_TARGETS_IN_PROMPT: usize = 10;

/// Executes a shell runner request triggered from Windows Explorer.
pub fn execute_runner(request: &RunnerRequest) -> i32 {
    let config = SDeleteConfig::load();
    let targets = match collect_batch_targets(request, config.sequential_execution) {
        Some(list) if !list.is_empty() => list,
        _ => return 0,
    };

    if config.confirm_before_delete && !confirm_execution(request, &targets, &config) {
        return 0;
    }

    let Some(sdelete_exe) = find_existing_sdelete() else {
        let strings = strings_for(config.language);
        show_error_box(strings.app_title, strings.sdelete_missing);
        return 1;
    };

    run_targets(&sdelete_exe, request, &targets, &config)
}

/// Collects multi-selected Explorer targets into one sequential batch via a named pipe.
fn collect_batch_targets(request: &RunnerRequest, sequential: bool) -> Option<Vec<String>> {
    if !sequential {
        return Some(vec![request.target_path.clone()]);
    }

    let batch_key = format!("{}_{}", request.mode.as_cli_token(), request.passes.get());
    let mutex_name = to_wide_null(&format!(r"Local\SDeleteGui_Batch_{batch_key}"));
    let pipe_path = format!(r"\\.\pipe\SDeleteGui_Pipe_{batch_key}");

    // SAFETY: CreateMutexW takes a valid wide string and returns a handle released before return.
    let mutex = unsafe { CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr()) };
    if mutex.is_null() {
        return Some(vec![request.target_path.clone()]);
    }

    // SAFETY: GetLastError reads the thread-local Win32 error code immediately after CreateMutexW.
    let already_exists = unsafe { GetLastError() == ERROR_ALREADY_EXISTS };
    if already_exists {
        // SAFETY: CloseHandle releases our duplicate handle to the existing mutex.
        unsafe { CloseHandle(mutex) };
        if forward_target_to_pipe(&pipe_path, &request.target_path) {
            return None;
        }
        return Some(vec![request.target_path.clone()]);
    }

    let mut targets = vec![request.target_path.clone()];
    let mut seen = HashSet::new();
    seen.insert(request.target_path.to_ascii_lowercase());

    receive_pipe_targets(&pipe_path, &mut targets, &mut seen);

    // SAFETY: This process owns the mutex; release and close it now.
    unsafe {
        ReleaseMutex(mutex);
        CloseHandle(mutex);
    }

    Some(targets)
}

fn forward_target_to_pipe(pipe_path: &str, target: &str) -> bool {
    let deadline = Instant::now() + Duration::from_millis(1200);
    while Instant::now() < deadline {
        if let Ok(mut pipe) = OpenOptions::new().write(true).open(pipe_path) {
            let payload = format!("{target}\n");
            if pipe.write_all(payload.as_bytes()).is_ok() {
                let _ = pipe.flush();
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

fn receive_pipe_targets(pipe_path: &str, targets: &mut Vec<String>, seen: &mut HashSet<String>) {
    let wide_pipe = to_wide_null(pipe_path);
    let total_deadline = Instant::now() + MAX_BATCH_WAIT;

    while Instant::now() < total_deadline {
        let remaining = total_deadline.saturating_duration_since(Instant::now());
        let wait_ms = remaining.min(BATCH_DEBOUNCE).as_millis().max(1) as u32;

        // SAFETY: CreateNamedPipeW returns INVALID_HANDLE_VALUE on error, checked immediately.
        let pipe = unsafe {
            CreateNamedPipeW(
                wide_pipe.as_ptr(),
                PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                0,
                4096,
                wait_ms,
                std::ptr::null(),
            )
        };
        if pipe == INVALID_HANDLE_VALUE || pipe.is_null() {
            break;
        }

        let connected = wait_for_pipe_connection(pipe, wait_ms);
        if connected {
            if let Some(line) = read_pipe_line(pipe) {
                let trimmed = line.trim();
                if !trimmed.is_empty() && seen.insert(trimmed.to_ascii_lowercase()) {
                    targets.push(trimmed.to_string());
                }
            }
            // SAFETY: DisconnectNamedPipe detaches the client before closing the handle.
            unsafe { DisconnectNamedPipe(pipe) };
            // SAFETY: pipe is a valid handle returned by CreateNamedPipeW.
            unsafe { CloseHandle(pipe) };
        } else {
            // SAFETY: pipe is a valid handle returned by CreateNamedPipeW.
            unsafe { CloseHandle(pipe) };
            break;
        }
    }
}

fn wait_for_pipe_connection(pipe: HANDLE, timeout_ms: u32) -> bool {
    // SAFETY: CreateEventW creates a manual-reset non-signaled event released before return.
    let event = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
    if event.is_null() {
        return false;
    }

    // SAFETY: OVERLAPPED is a plain C struct safe to zero-initialize.
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    overlapped.hEvent = event;

    // SAFETY: ConnectNamedPipe uses valid pipe and overlapped pointers.
    let rc = unsafe { ConnectNamedPipe(pipe, &mut overlapped) };
    let connected = if rc != 0 {
        true
    } else {
        // SAFETY: GetLastError reads the error status of ConnectNamedPipe.
        let err = unsafe { GetLastError() };
        if err == ERROR_PIPE_CONNECTED {
            true
        } else if err == ERROR_IO_PENDING {
            // SAFETY: event is valid and remains alive until CancelIoEx completes.
            let wait = unsafe { WaitForSingleObject(event, timeout_ms) };
            if wait == WAIT_OBJECT_0 {
                let mut transferred = 0u32;
                // SAFETY: pipe and overlapped are valid and I/O signaled completion.
                unsafe { GetOverlappedResult(pipe, &overlapped, &mut transferred, 0) != 0 }
            } else {
                // SAFETY: CancelIoEx cancels the pending overlapped ConnectNamedPipe before stack unwind.
                unsafe {
                    CancelIoEx(pipe, &overlapped);
                    let mut transferred = 0u32;
                    GetOverlappedResult(pipe, &overlapped, &mut transferred, 1);
                }
                false
            }
        } else {
            false
        }
    };

    // SAFETY: event is a valid handle from CreateEventW.
    unsafe { CloseHandle(event) };
    connected
}

fn read_pipe_line(pipe: HANDLE) -> Option<String> {
    let mut buf = [0u8; 4096];
    let mut bytes_read = 0u32;
    // SAFETY: CreateEventW creates a valid event handle closed before return.
    let event = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
    if event.is_null() {
        return None;
    }

    // SAFETY: OVERLAPPED is a plain C struct safe to zero-initialize.
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    overlapped.hEvent = event;

    // SAFETY: ReadFile writes at most buf.len() bytes into buf.
    let ok = unsafe {
        ReadFile(
            pipe,
            buf.as_mut_ptr(),
            buf.len() as u32,
            &mut bytes_read,
            &mut overlapped,
        )
    };

    if ok == 0 {
        // SAFETY: Check pending I/O and wait briefly for client payload.
        let err = unsafe { GetLastError() };
        if err == ERROR_IO_PENDING {
            let wait = unsafe { WaitForSingleObject(event, 500) };
            if wait == WAIT_OBJECT_0 {
                unsafe {
                    GetOverlappedResult(pipe, &overlapped, &mut bytes_read, 0);
                }
            } else {
                unsafe {
                    CancelIoEx(pipe, &overlapped);
                    GetOverlappedResult(pipe, &overlapped, &mut bytes_read, 1);
                }
            }
        }
    }

    // SAFETY: CloseHandle releases the event handle.
    unsafe { CloseHandle(event) };

    if bytes_read == 0 {
        return None;
    }

    let text = String::from_utf8_lossy(&buf[..bytes_read as usize]);
    text.lines().next().map(ToString::to_string)
}

/// Shows a native Windows confirmation dialog before running SDelete.
fn confirm_execution(request: &RunnerRequest, targets: &[String], config: &SDeleteConfig) -> bool {
    let strings = strings_for(config.language);
    let mut body = String::new();

    match request.mode {
        SDeleteMode::Delete => {
            body.push_str(&format!(
                "Permanently erase {} item(s) with {} pass(es)?\n\n",
                targets.len(),
                request.passes.get()
            ));
        }
        SDeleteMode::CleanFreeSpace => {
            body.push_str(&format!(
                "Run {} (-c) on drive {} with {} pass(es)?\n\n",
                strings.clean_free_space_short,
                targets[0],
                request.passes.get()
            ));
        }
        SDeleteMode::ZeroFreeSpace => {
            body.push_str(&format!(
                "Run {} (-z) on drive {} with {} pass(es)?\n\n",
                strings.zero_free_space_short,
                targets[0],
                request.passes.get()
            ));
        }
    }

    for path in targets.iter().take(MAX_TARGETS_IN_PROMPT) {
        body.push_str(" - ");
        body.push_str(path);
        body.push('\n');
    }

    if targets.len() > MAX_TARGETS_IN_PROMPT {
        body.push_str(&format!(
            "{}{}{}\n",
            strings.confirm_more_prefix,
            targets.len() - MAX_TARGETS_IN_PROMPT,
            strings.confirm_more_suffix
        ));
    }

    body.push('\n');
    body.push_str(strings.confirm_irreversible);

    let wide_title = to_wide_null(strings.confirm_title);
    let wide_body = to_wide_null(&body);

    // SAFETY: MessageBoxW accepts valid null-terminated UTF-16 strings.
    let result = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide_body.as_ptr(),
            wide_title.as_ptr(),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2 | MB_TOPMOST,
        )
    };
    result == IDYES
}

fn show_error_box(title: &str, message: &str) {
    let wide_title = to_wide_null(title);
    let wide_body = to_wide_null(message);
    // SAFETY: MessageBoxW accepts valid null-terminated UTF-16 strings.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide_body.as_ptr(),
            wide_title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_TOPMOST,
        );
    }
}

/// Runs Sysinternals sdelete64.exe sequentially (or detached if sequential mode is off).
pub fn run_targets(
    sdelete_exe: &Path,
    request: &RunnerRequest,
    targets: &[String],
    config: &SDeleteConfig,
) -> i32 {
    let mut worst_exit = 0;

    for target in targets {
        let args = build_sdelete_args(request.mode, request.passes, target);
        let (exit_code, error_detail) = if config.sequential_execution {
            let mut cmd = Command::new(sdelete_exe);
            cmd.args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW);

            match cmd.spawn() {
                Ok(mut child) => {
                    let mut stderr_text = String::new();
                    if let Some(mut err_pipe) = child.stderr.take() {
                        let _ = err_pipe.read_to_string(&mut stderr_text);
                    }
                    match child.wait() {
                        Ok(status) => {
                            let code = status.code().unwrap_or(-1);
                            let detail = if code != 0 && !stderr_text.trim().is_empty() {
                                Some(stderr_text)
                            } else {
                                None
                            };
                            (code, detail)
                        }
                        Err(err) => (-1, Some(err.to_string())),
                    }
                }
                Err(err) => (-1, Some(err.to_string())),
            }
        } else {
            let mut cmd = Command::new(sdelete_exe);
            cmd.args(&args);
            match cmd.spawn() {
                Ok(_) => (0, None),
                Err(err) => (-1, Some(err.to_string())),
            }
        };

        if exit_code != 0 {
            worst_exit = exit_code;
        }

        if config.enable_audit_log {
            let _ = append_entry(
                &config.log_file_path,
                request.mode,
                request.passes,
                target,
                exit_code,
                error_detail.as_deref(),
            );
        }
    }

    worst_exit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PassCount;

    #[test]
    fn named_pipe_batch_collector_merges_concurrent_targets() {
        let pipe_path = format!(r"\\.\pipe\SDeleteGui_TestPipe_{}", std::process::id());
        let pipe_for_client = pipe_path.clone();

        let client = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(60));
            assert!(forward_target_to_pipe(
                &pipe_for_client,
                r"C:\Temp\second.txt"
            ));
        });

        let mut targets = vec![r"C:\Temp\first.txt".to_string()];
        let mut seen = HashSet::new();
        seen.insert(r"c:\temp\first.txt".to_string());
        receive_pipe_targets(&pipe_path, &mut targets, &mut seen);

        client.join().unwrap();
        assert_eq!(targets, vec![r"C:\Temp\first.txt", r"C:\Temp\second.txt"]);
    }

    #[test]
    fn run_targets_executes_sequentially_and_writes_audit_log() {
        let temp_dir = std::env::temp_dir().join(format!("sdelete_gui_run_{}", std::process::id()));
        let log_file = temp_dir.join("run_audit.log");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let cfg = SDeleteConfig {
            sequential_execution: true,
            enable_audit_log: true,
            log_file_path: log_file.clone(),
            ..Default::default()
        };

        let req = RunnerRequest {
            mode: SDeleteMode::Delete,
            passes: PassCount::new(1).unwrap(),
            target_path: r"C:\Temp\alpha.txt".to_string(),
        };

        // Invoke cmd.exe /C exit 0? Here we point to cmd.exe with non-zero args? Instead use a tiny batch stub.
        std::fs::create_dir_all(&temp_dir).unwrap();
        let stub = temp_dir.join("stub_sdelete.cmd");
        std::fs::write(&stub, "@echo off\r\nexit /b 0\r\n").unwrap();

        let targets = vec![
            r"C:\Temp\alpha.txt".to_string(),
            r"C:\Temp\beta.txt".to_string(),
        ];
        let code = run_targets(&stub, &req, &targets, &cfg);
        assert_eq!(code, 0);

        let log_content = std::fs::read_to_string(&log_file).unwrap();
        let lines: Vec<&str> = log_content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("alpha.txt") && lines[0].contains("STATUS=OK"));
        assert!(lines[1].contains("beta.txt") && lines[1].contains("STATUS=OK"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
