# Mouse Mover App

A pure Rust mouse movement tool using native Win32 controls. Runs directly without .NET or a bundled GUI runtime.

The Rust build is about 342 KB; local warm startup was about 10× faster than the
previous C# standalone release. See [measurements and verification](docs/rust-migration-results.md).

## Download and usage

Download `MouseMoverApp.exe` from [Releases](https://github.com/yaosio232/mouse_mover/releases) and run it directly. Supports Windows x64.

- Press **Start (F10)** or the global **F10** hotkey to start monitoring physical mouse and keyboard input.
- After three minutes of inactivity, the cursor starts moving. When movement finishes, the app clicks the left mouse button once, waits 60 seconds, and repeats.
- Keyboard input or mouse movement cancels automatic movement until the next three-minute idle period.
- Press **Stop** or **F10** to stop completely.

## Additional features

- **Stop countdown** is an independent stop timer. Enabling it or changing the duration restarts the countdown immediately; pressing Start also restarts it. When it expires, the app displays Done and stops completely. It is disabled by default, with a preset of 3 hours / 0 minutes / 0 seconds.
- Closing the window stops the app. There is no background service, saved settings, or network functionality.

## Build and publish

Requires Windows, Rust stable (x86_64-pc-windows-gnu), and a compatible x64 MinGW-w64 toolchain (GCC, dlltool, windres) on PATH:

```powershell
cargo build --release --locked
cargo test --locked
./scripts/publish.ps1
```

Use `cargo fmt --check` and `cargo clippy --locked --all-targets -- -D warnings`
for source checks. After publishing, run
`./scripts/check-ui.ps1 -Executable artifacts/release/MouseMoverApp.exe` on Windows.
The publish directory must be empty; choose `-OutputDirectory artifacts/rust-release`
if an older release already exists. The original four specifications describe the
C# baseline; the current implementation and build are entirely Rust.

The publish directory `artifacts/release` contains `MouseMoverApp.exe` and English / Traditional Chinese instruction TXT files (`README_EN.txt` and `README_ZH-TW.txt`).

---

## 中文

全 Rust 滑鼠散步工具，使用原生 Win32 控制項，不需要 .NET 或額外 GUI 執行環境。

### 下載與操作

從 [Releases](https://github.com/yaosio232/mouse_mover/releases) 下載 `MouseMoverApp.exe`，直接執行。支援 Windows x64。

- 按 **Start (F10)** 或全域 **F10**，開始監看真實滑鼠及鍵盤輸入。
- 閒置三分鐘後，游標會開始移動，完成後左鍵點擊一次，等待 60 秒再重複。
- 鍵盤輸入或滑鼠移動時會取消移動，直到再度閒置三分鐘。
- 按 **Stop** 或 **F10** 完全停止。

### 額外功能

- **Stop countdown** 是獨立的停止倒數；勾選／修改時長立即重新計時，Start 時也重新計時，到期顯示 Done 並完全停止。預設未勾選，3 小時／0 分鐘／0 秒。
- 關閉視窗即停止；沒有背景服務、設定儲存或網路功能。

### 建置與發佈

需要 Windows、Rust stable（x86_64-pc-windows-gnu），以及 PATH 中相容的 x64 MinGW-w64 工具鏈（GCC、dlltool、windres）：

```powershell
cargo build --release --locked
cargo test --locked
./scripts/publish.ps1
```

程式碼檢查：`cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings`。
發佈後可執行 `./scripts/check-ui.ps1 -Executable artifacts/release/MouseMoverApp.exe`
驗證原生視窗及操作。發佈目錄必須為空；若已有舊版，可加上
`-OutputDirectory artifacts/rust-release`。原始四份規格保留作為 C# 版行為基準，
目前實作與建置已改為全 Rust。

發佈目錄 `artifacts/release` 含 `MouseMoverApp.exe` 及中英文說明 TXT（`README_EN.txt` 與 `README_ZH-TW.txt`）。
