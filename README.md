# SDelete Gui (Rust)

Windows Explorer context-menu manager and sequential batch runner for [Microsoft Sysinternals SDelete](https://learn.microsoft.com/en-us/sysinternals/downloads/sdelete).

## Features

- **Sysinternals NTFS parity:** Wraps Microsoft Sysinternals `sdelete64.exe` (downloads automatically via HTTPS if not present locally).
- **Deletion confirmation dialog:** Displays selected targets, mode, pass count, and an irreversibility warning before deletion.
- **Sequential multi-file execution:** Merges multi-selected Explorer items over a Windows named pipe and executes `sdelete64.exe` sequentially without opening parallel console windows.
- **Pass-count cascading submenu:** Adds a grouped Explorer menu with the configured default pass count plus quick presets (`1`, `3`, `5`, `7`, and `35` passes).
- **Warning context-menu icon:** Adds an optional warning icon (`imageres.dll,-84`) to context-menu entries.
- **Separate drive free-space modes:** Exposes **Clean Free Space (`-c`)** and **Zero Free Space (`-z`)** as distinct drive context-menu commands.
- **Timestamped audit log:** Appends ISO-8601 timestamped execution records to a configurable log file under a cross-process mutex.
- **No-elevation Current User scope:** Registers under `HKCU\Software\Classes` by default, with optional `All Users` (`HKLM\Software\Classes`) support when run as Administrator.
- **Dark and light themes:** Includes a built-in Dark/Light theme toggle and English, Spanish, and German UI localization.

## Build and Test

Requires Rust (`stable-x86_64-pc-windows-msvc`) and Visual Studio Build Tools on Windows:

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

The standalone executable builds to `target\release\sdelete-gui.exe`.

## Storage Caveats

- Software overwrites cannot guarantee physical flash erasure on SSDs that use wear-leveling and TRIM.
- Sysinternals SDelete clears unallocated free space (`-c` / `-z`), not live-file cluster slack space.

## License

MIT License. Refer to `LICENSE.txt`.
