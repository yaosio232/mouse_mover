# AZu Sanpo

原生 Windows Forms 滑鼠散步工具，依此倉庫的四份規格重建。

## 下載與操作

從 [Releases](https://github.com/yaosio232/mouse_mover/releases) 下載 `MouseMoverApp.exe`，直接執行。支援 Windows x64，已包含 .NET 執行環境，無須安裝 .NET 或額外 DLL。

- 按 **Start (F10)** 或全域 **F10**，開始監看真實滑鼠及鍵盤輸入。
- 連續閒置三分鐘，游標走 AZKi 愛心路徑（446 個採樣點），完成後左鍵點擊一次，等待 60 秒再重複。
- 真實輸入會取消當輪移動，重新等待三分鐘。按 Stop 或 F10 完全停止。
- **Stop countdown** 是獨立的停止倒數；勾選／修改時長立即重新計時，Start 時也重新計時，到期顯示 Done 並完全停止。預設未勾選，180 分鐘／0 秒。
- 關閉視窗即停止；沒有背景服務、設定儲存或網路功能。

## 建置與發佈

Windows 與 .NET 8 SDK：

```powershell
dotnet build -c Release
dotnet run --project tests/MouseMoverApp.Checks -c Release
./scripts/publish.ps1
```

發佈目錄 `artifacts/release` 只含一個 `MouseMoverApp.exe`。SHA256 位於上一層 `artifacts/SHA256SUMS.txt`，不會混入 EXE 交付目錄。輸出目錄須為空；重建可指定 `-OutputDirectory artifacts/release-new`，不刪除舊檔。單檔 EXE 可能在使用者暫存目錄解壓原生元件。

完整檢查會短暫建立視窗、測試全域 F10、安裝真實 hook，並在程式自己的狀態標籤上模擬點擊。執行期間請盡量不要操作滑鼠／鍵盤。純邏輯檢查可加 `-- --core`，CI 使用此模式。完整硬體輸入驗收清單見 [驗證記錄](docs/verification.md)。

## 與歷史基準的差異

- 使用本機現有 .NET 8 SDK，取代規格記錄的 .NET 6；WinForms 與 Win32 行為保持相同。
- 修正规格的鍵盤注入判定：鍵盤使用 `LLKHF_INJECTED (0x10)`；滑鼠仍使用 `0x01`。確保實體延伸鍵也會重設閒置，模擬鍵盤輸入不會中斷移動。依 [Microsoft API 文件](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-kbdllhookstruct)。
- 圖示依使用者提供的 rat SVG 製作，rat 與外圈均為藍色，背景透明；保留 SVG、PNG 與多尺寸 ICO。
- 保留 Random 策略類別供修改編譯設定，UI 固定使用 AZKi。

原始規格：[功能與狀態](01-FUNCTIONAL-SPEC.md)、[UI](02-UI-SPEC.md)、[路徑](03-MOVEMENT-SPEC.md)、[平台與驗收](04-PLATFORM-QA.md)。

倉庫為公開倉庫；不得提交憑證、Token、私鑰、個人設定或建置中間檔。
