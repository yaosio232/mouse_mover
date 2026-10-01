# Mouse Mover App

A native Windows Forms mouse movement tool, rebuilt from the four specifications in this repository.

## Download and usage

Download `MouseMoverApp.exe` from [Releases](https://github.com/yaosio232/mouse_mover/releases) and run it directly. Supports Windows x64.

- Press **Start (F10)** or the global **F10** hotkey to start monitoring physical mouse and keyboard input.
- After three minutes of inactivity, the cursor starts moving. When movement finishes, the app clicks the left mouse button once, waits 60 seconds, and repeats.
- Keyboard input or mouse movement cancels automatic movement until the next three-minute idle period.
- Press **Stop** or **F10** to stop completely.

## Additional features

- **Stop countdown** is an independent stop timer. Enabling it or changing the duration restarts the countdown immediately; pressing Start also restarts it. When it expires, the app displays Done and stops completely. It is disabled by default, with a preset of 180 minutes / 0 seconds.
- Closing the window stops the app. There is no background service, saved settings, or network functionality.

## Build and publish

Requires Windows and the .NET 8 SDK:

```powershell
dotnet build -c Release
dotnet run --project tests/MouseMoverApp.Checks -c Release
./scripts/publish.ps1
```

The publish directory `artifacts/release` contains `MouseMoverApp.exe` and English / Traditional Chinese instruction TXT files (`README_EN.txt` and `README_ZH-TW.txt`).

---

## 中文

原生 Windows Forms 滑鼠散步工具，依此倉庫的四份規格重建。

### 下載與操作

從 [Releases](https://github.com/yaosio232/mouse_mover/releases) 下載 `MouseMoverApp.exe`，直接執行。支援 Windows x64。

- 按 **Start (F10)** 或全域 **F10**，開始監看真實滑鼠及鍵盤輸入。
- 閒置三分鐘後，游標會開始移動，完成後左鍵點擊一次，等待 60 秒再重複。
- 鍵盤輸入或滑鼠移動時會取消移動，直到再度閒置三分鐘。
- 按 **Stop** 或 **F10** 完全停止。

### 額外功能

- **Stop countdown** 是獨立的停止倒數；勾選／修改時長立即重新計時，Start 時也重新計時，到期顯示 Done 並完全停止。預設未勾選，180 分鐘／0 秒。
- 關閉視窗即停止；沒有背景服務、設定儲存或網路功能。

### 建置與發佈

Windows 與 .NET 8 SDK：

```powershell
dotnet build -c Release
dotnet run --project tests/MouseMoverApp.Checks -c Release
./scripts/publish.ps1
```

發佈目錄 `artifacts/release` 含 `MouseMoverApp.exe` 及中英文說明 TXT（`README_EN.txt` 與 `README_ZH-TW.txt`）。
