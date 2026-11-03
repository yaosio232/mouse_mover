Mouse Mover App

A pure Rust mouse movement tool using native Win32 controls. Runs directly without .NET or a bundled GUI runtime.

Download and usage

Download MouseMoverApp.exe from https://github.com/yaosio232/mouse_mover/releases and run it directly. Supports Windows x64.

- Press Start (F10) or the global F10 hotkey to start monitoring physical mouse and keyboard input.
- After three minutes of inactivity by default, the cursor starts moving. When movement finishes, the app clicks the left mouse button once, waits 60 seconds by default, and repeats.
- Keyboard input or mouse movement cancels automatic movement until the configured idle period passes again.
- Press Stop or F10 to stop completely.

Additional features

- Stop after is an independent stop timer. Enabling it or changing the duration restarts the countdown immediately; pressing Start also restarts it. When it expires, the app displays Stop done and stops completely. It is disabled by default, with a preset of 3 hours / 0 minutes / 0 seconds.
- The gear sets idle time (1/3/5/10 minutes), repeat interval (30/60/120/300 seconds), background transparency, and writing or random movement. Random mode moves to 3-5 positions before clicking the screen center. Settings are saved locally.
- Closing the window stops the app. There is no background service or network functionality.

Build and publish

Requires Windows, Rust stable (x86_64-pc-windows-gnu), and a compatible x64 MinGW-w64 toolchain (GCC, dlltool, windres) on PATH:

cargo +stable-x86_64-pc-windows-gnu build --release --locked
cargo test --locked
./scripts/publish.ps1

The publish directory artifacts/release contains MouseMoverApp.exe, README_EN.txt, and README_ZH-TW.txt.
