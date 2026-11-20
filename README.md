# SDelete Gui (Rust)

Securely delete files, folders, and drive free space with a Windows Explorer right-click menu.

## Introduction

**SDelete Gui** is a Windows utility that adds, configures, and removes a "Secure Delete" menu in the Windows Explorer context menu for files, folders, and drives. Secure Delete permanently overwrites and deletes the selected files and folders so recovery tools cannot restore them.

By default, this version displays a confirmation dialog that lists the selected targets, mode, and pass count before deletion starts. You can disable this prompt in the configuration window if you prefer immediate deletion.

This application manages the context menu, batch queue, confirmation dialog, and audit log, but it does not implement its own low-level overwrite driver. It wraps [SDelete](https://learn.microsoft.com/en-us/sysinternals/downloads/sdelete) from [Microsoft Sysinternals](https://learn.microsoft.com/en-us/sysinternals/), written by Mark Russinovich, to preserve exact Sysinternals NTFS behavior with a United States Department of Defense compliant overwrite algorithm. Refer to [How SDelete Works](https://learn.microsoft.com/en-us/sysinternals/downloads/sdelete#how-sdelete-works) for technical details.

When you click **Enable**, the application checks `%LOCALAPPDATA%\SDelete-Gui`, `System32`, `SysWOW64`, and `PATH` for `sdelete64.exe` (or `sdelete.exe`). If no local copy exists, it downloads the official binary over HTTPS from `live.sysinternals.com`. If the download fails and no local binary exists, the application displays an error message.

## Features

- **Sysinternals NTFS parity:** Wraps Microsoft Sysinternals `sdelete64.exe` and downloads it via HTTPS when absent locally.
- **Deletion confirmation dialog:** Displays selected targets, mode, pass count, and an irreversibility warning before deletion.
- **Sequential multi-file execution:** Merges multi-selected Explorer items over a Windows named pipe and runs `sdelete64.exe` sequentially without opening parallel console windows.
- **Pass-count cascading submenu:** Adds a grouped Explorer menu with the configured default pass count plus quick presets (`1`, `3`, `5`, `7`, and `35` passes).
- **Warning context-menu icon:** Adds an optional warning icon (`imageres.dll,-84`) to context-menu entries.
- **Separate drive free-space modes:** Exposes **Clean Free Space (`-c`)** and **Zero Free Space (`-z`)** as separate drive context-menu commands.
- **Timestamped audit log:** Appends ISO-8601 timestamped execution records to a configurable log file under a cross-process mutex.
- **Current User and All Users scopes:** Registers under `HKCU\Software\Classes` without elevation by default, or under `HKLM\Software\Classes` when run as Administrator.
- **Dark and light themes:** Provides a built-in Dark/Light theme toggle and English, Spanish, and German UI localization.

## How to Use

1. Download or build `sdelete-gui.exe` (refer to **Build and Test** below).
2. Run `sdelete-gui.exe` to open the configuration window.
3. Set the default number of overwrite passes (`1` to `100`).
4. Select your preferred options: confirmation prompt, sequential multi-file deletion, pass-count submenu, warning icon, and timestamped audit log path.
5. Select **Current user (no elevation)** for your account only, or run `sdelete-gui.exe` as Administrator and select **All users (administrator)**.
6. Click **Enable** to save settings, ensure `sdelete64.exe` is present, and register the Windows Explorer context menus.
7. Right-click files, folders, or drives in Windows Explorer and select **Secure Delete** or **Erase Free Space (SDelete)**.
8. Click **Disable** in the configuration window at any time to remove the context-menu entries and managed local binary.

## Build and Test

Install Rust (`stable-x86_64-pc-windows-msvc`) and Visual Studio Build Tools on Windows, then run:

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

The standalone executable builds to `target\release\sdelete-gui.exe`.

## Storage Caveats

- Software overwrites cannot guarantee physical flash erasure on SSDs that use wear-leveling and TRIM.
- Sysinternals SDelete clears unallocated free space (`-c` or `-z`), not live-file cluster slack space.

## Attribution and Credits

- Original concept, design, and C# / WPF implementation by **Tulpep** at [Tulpep/SDelete-Gui](https://github.com/Tulpep/SDelete-Gui).
- Underlying secure-deletion engine by **Mark Russinovich** and **Microsoft Sysinternals** ([SDelete](https://learn.microsoft.com/en-us/sysinternals/downloads/sdelete)).

## License

MIT License. Refer to `LICENSE.txt`.
