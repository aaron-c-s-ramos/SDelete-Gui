use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use crate::config::RegistryScope;

/// Official Microsoft Sysinternals download URL for 64-bit SDelete.
pub const SDELETE64_URL: &str = "https://live.sysinternals.com/sdelete64.exe";

/// Official Microsoft Sysinternals download URL for 32-bit SDelete.
pub const SDELETE32_URL: &str = "https://live.sysinternals.com/sdelete.exe";

/// Errors produced when locating or downloading Sysinternals SDelete.
#[derive(Debug, Error)]
pub enum BinaryError {
    #[error("Failed to download Sysinternals SDelete from {url}: {source}")]
    Download {
        url: String,
        #[source]
        source: Box<ureq::Error>,
    },
    #[error("Filesystem error while saving Sysinternals SDelete: {0}")]
    Io(#[from] io::Error),
}

/// Returns true when the current process runs with Administrator privileges.
pub fn is_process_elevated() -> bool {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: GetCurrentProcess returns a pseudo-handle; OpenProcessToken populates token on success.
    let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if opened == 0 || token.is_null() {
        return false;
    }

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut out_len: u32 = 0;
    // SAFETY: token is valid and elevation has exact size of TOKEN_ELEVATION.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut out_len,
        )
    };
    // SAFETY: CloseHandle releases the token handle returned by OpenProcessToken.
    unsafe { CloseHandle(token) };

    ok != 0 && elevation.TokenIsElevated != 0
}

/// Returns the preferred local storage path for the downloaded Sysinternals binary.
pub fn preferred_install_path(scope: RegistryScope) -> PathBuf {
    let binary_name = if cfg!(target_pointer_width = "64") {
        "sdelete64.exe"
    } else {
        "sdelete.exe"
    };

    if scope == RegistryScope::AllUsers && is_process_elevated() {
        if let Some(windir) = std::env::var_os("WINDIR") {
            return PathBuf::from(windir).join("System32").join(binary_name);
        }
    }

    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("SDelete-Gui").join(binary_name)
}

/// Searches standard local folders and PATH for an existing sdelete64.exe or sdelete.exe.
pub fn find_existing_sdelete() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        let dir = PathBuf::from(local_app_data).join("SDelete-Gui");
        candidates.push(dir.join("sdelete64.exe"));
        candidates.push(dir.join("sdelete.exe"));
    }

    if let Some(windir) = std::env::var_os("WINDIR") {
        let win = PathBuf::from(windir);
        candidates.push(win.join("System32").join("sdelete64.exe"));
        candidates.push(win.join("System32").join("sdelete.exe"));
        candidates.push(win.join("SysWOW64").join("sdelete64.exe"));
        candidates.push(win.join("SysWOW64").join("sdelete.exe"));
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            candidates.push(dir.join("sdelete64.exe"));
            candidates.push(dir.join("sdelete.exe"));
        }
    }

    candidates.into_iter().find(|p| p.is_file())
}

/// Ensures Sysinternals SDelete is available locally, downloading it via HTTPS when absent.
pub fn ensure_sdelete_binary(scope: RegistryScope) -> Result<PathBuf, BinaryError> {
    if let Some(existing) = find_existing_sdelete() {
        return Ok(existing);
    }

    let dest = preferred_install_path(scope);
    download_sdelete(&dest)?;
    Ok(dest)
}

/// Downloads Microsoft Sysinternals SDelete over HTTPS to the target path.
pub fn download_sdelete(destination: &Path) -> Result<(), BinaryError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let url = if cfg!(target_pointer_width = "64") {
        SDELETE64_URL
    } else {
        SDELETE32_URL
    };

    let response = ureq::get(url)
        .call()
        .map_err(|source| BinaryError::Download {
            url: url.to_string(),
            source: Box::new(source),
        })?;

    let temp_path = destination.with_extension("exe.tmp");
    {
        let mut reader = response.into_body().into_reader();
        let mut file = File::create(&temp_path)?;
        io::copy(&mut reader, &mut file)?;
        file.sync_all()?;
    }

    fs::rename(&temp_path, destination)?;
    Ok(())
}

/// Removes the managed SDelete binary from the per-user folder if present.
pub fn remove_managed_binary() -> Result<(), io::Error> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("SDelete-Gui");
    for name in ["sdelete64.exe", "sdelete.exe"] {
        let path = dir.join(name);
        if path.is_file() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}
