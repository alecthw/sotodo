# So Todo

Windows-only todo app built with Dioxus 0.7.

## Features

- Todo list, calendar view, and completed view
- Optional due date, due time, and reminder minutes
- Date-only todos are grouped under that day; todos without date/time are grouped separately
- Weekly and monthly recurring todos
- Overdue marking for unfinished todos past their due time
- Windows toast notifications before due time and at due time
- System tray: show main window and exit
- Optional start with Windows
- Light/dark follow system by default, plus DaisyUI themes
- Chinese/English UI with system language detection
- SQLite single-file storage
- Runtime output can be a single `sotodo.exe`

## Data

The database is stored at:

```text
%USERPROFILE%\.sotodo\sotodo.db
```

SQLite is bundled through `rusqlite` with the `bundled` feature, so no external SQLite install is required.

## Development

Install Rust and Dioxus CLI, then run:

```powershell
dx serve --platform desktop
```

Run tests:

```powershell
cargo test
```

Build the Windows desktop output:

```powershell
dx build --platform desktop
```

The output is generated at:

```text
target\dx\sotodo\debug\windows\app\sotodo.exe
```

## Single Exe

Runtime assets are embedded into the executable. The `assets/` directory is still needed at compile time for CSS and icon inputs, but the built app does not need to ship with an `assets/` folder.

## Project Layout

```text
assets/        compile-time CSS and icon inputs
src/main.rs    application UI, state, storage, tray, startup, notifications
Cargo.toml     Rust dependencies and features
Dioxus.toml    Dioxus bundle metadata
```
