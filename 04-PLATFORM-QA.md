# MouseMoverApp 重建規格：平台、發佈與驗收

> 本文件提供另一個 Agent 的實作對照和完成條件。功能狀態以 [01-FUNCTIONAL-SPEC.md](01-FUNCTIONAL-SPEC.md) 為準；UI 以 [02-UI-SPEC.md](02-UI-SPEC.md) 及其原程式圖片為準；游標路徑以 [03-MOVEMENT-SPEC.md](03-MOVEMENT-SPEC.md) 為準。不要把保留的 Random 內部策略做成新 UI 功能，也不要把 `Start` 改成直接移動。

## 1. 原始技術組成

| 項目 | 目前實作 |
| --- | --- |
| 專案形式 | C# Windows Forms `WinExe`，`.NET 6`，`net6.0-windows`。 |
| 目標平台 | `win-x64`，自包含、單檔發佈，不啟用 trimming。 |
| Visual Styles／啟動 | `Program.Main` 標記 `[STAThread]`，呼叫 `ApplicationConfiguration.Initialize()` 後 `Application.Run(new Form1())`。 |
| UI 主體 | `Form1.Designer.cs` 建立原生控制項；`Form1.cs` 持有狀態、事件及循環。 |
| 輸入監看 | `PhysicalInputMonitor.cs`，Win32 低階滑鼠與鍵盤 hook。 |
| 移動策略 | `IMouseMoveStrategy.cs`、`MouseMoveStrategyContext.cs`、`AzkiMouseMoveStrategy.cs`、`RandomMouseMoveStrategy.cs`。 |
| 應用程式圖示 | `mouseIcon.ico` 設為 `ApplicationIcon`。 |
| 不需的外部依賴 | 專案沒有 NuGet UI 套件、資料庫、伺服器、帳號或網路 API。 |

若新 Agent 改用別的 UI 框架，仍應以第 02 份的原生 Windows 控制項外觀和參考圖驗收，而非依新框架預設主題自行設計。

## 2. Windows API 契約

| API／常數 | 用途與對等要求 |
| --- | --- |
| `RegisterHotKey(handle, 1, 0, 0x79)` | 註冊無修飾鍵全域 F10。`WM_HOTKEY=0x0312` 且 `WParam=1` 時執行與主按鈕相同的 toggle。handle 銷毀時呼叫 `UnregisterHotKey`。 |
| `SetWindowPos(handle, HWND_TOPMOST=-1, ..., SWP_NOACTIVATE=0x0010 \| SWP_SHOWWINDOW=0x0040)` | Form Load 時置頂而不因此啟用；Form 的 `TopMost` 也設為 true。 |
| `SetForegroundWindow(handle)` | 達到閒置門檻而進入自動移動時，配合 `Show()`、`Activate()` 與 `WindowState=Normal` 嘗試將視窗帶至前景。 |
| `GetLastInputInfo` | 在剛開始監看時計算當前 session 已有閒置時間；在完全停止狀態更新 UI 時仍取系統閒置時間。32 位 tick 差使用 `uint` 相減。 |
| `SetWindowsHookEx(WH_MOUSE_LL=14 / WH_KEYBOARD_LL=13)` | 監看真實輸入。兩個 hook 均以目前 UI 執行緒註冊，該執行緒需有 WinForms 訊息迴圈；delegate 必須被實例欄位持有，避免 GC。停止或關閉時 `UnhookWindowsHookEx`。 |
| `MSLLHOOKSTRUCT.flags` / `KBDLLHOOKSTRUCT.flags` | `flags & 1` 不為零即注入事件，不觸發使用者活動回呼；其他事件更新最後輸入 tick。callback 必須傳給 `CallNextHookEx`，不能吞掉其他程式的輸入。 |
| `Cursor.Position` | 相對游標移動的實際位置讀取與設置。不要改成直接發送 `WM_MOUSEMOVE` 給本視窗；移動要作用於系統游標。 |
| `mouse_event(0x02 \| 0x04, 0, 0, 0, 0)` | 每輪策略完成後在目前游標位置模擬一次左鍵按下加放開。用單次呼叫、組合旗標；不根據路徑逐點點擊。 |

低階 hook 必須排除自己注入的點擊；目前環境實測程式設置 `Cursor.Position` 也未被當成真實活動。若新實作採用不同 Windows 輸入 API，仍須驗證這兩種自行產生的事件不會讓移動立刻進入 Monitoring。

## 3. 時序與例外細節

- `idleTimer` 只在 `isIdleMonitoring || isMouseMoving || stopCountdownEndAt.HasValue` 時運作，tick 間隔 1000 ms。每 tick 先更新／檢查停止倒數，再更新／檢查閒置狀態。
- 移動工作在背景 Task，UI timer、hook callback 與 UI 控制項更新由 WinForms UI 執行緒處理。取消透過 `CancellationTokenSource`，路徑每點／每步、延遲和點擊前均要有取消安全點。
- 停止倒數截止時間使用當地 `DateTime.Now` 加上設定時長；每 tick 以 `endAt - DateTime.Now` 計算剩餘。閒置三分鐘則使用不受系統時鐘調整影響的 tick clock，兩種時鐘不可混用。
- 若 hook 安裝失敗，顯示 Windows 原生 Error MessageBox 且不進入監看；若移動循環拋出非取消例外，完整停止並顯示原生 Error MessageBox。這兩種情況的精確文字在第 01 份。
- 目前沒有將設定寫至磁碟。每次啟動皆為未勾選停止倒數、`180 min`、`0 sec`、尚未開始監看、AZKi 策略。

## 4. 對照原碼的檔案清單

重建時不一定要用相同的檔名，但需求追溯時請對照下列來源；目前工作目錄的檔案內容已包含尚未提交的修改。

| 原始檔案 | 可對照內容 |
| --- | --- |
| `Form1.Designer.cs` | 視窗屬性、控制項型別、座標、設計尺寸、TabIndex。 |
| `Form1.cs` | 常數、UI 字串、狀態轉移、閒置與停止倒數、移動循環、Windows API 呼叫。 |
| `PhysicalInputMonitor.cs` | 低階輸入 hook、注入事件過濾、解除 hook。 |
| `MouseMoveStrategyContext.cs` | 相對位移累積、Math.Round、每步取消、邊界裁切。 |
| `AzkiMouseMoveStrategy.cs` | 446 點路徑及各採樣公式。 |
| `RandomMouseMoveStrategy.cs` | 未啟用的替代模式。 |
| `MouseMoverApp.csproj` / `Program.cs` | 目標框架、發佈設定、啟動流程。 |
| `mouseIcon.ico` | EXE 圖示。 |
| `smoke_test.ps1` | 原程式的程式化狀態／合成事件檢查腳本，可用作重建測試情境參照。 |

## 5. 建置與真正單檔輸出

目前專案可用以下命令建置：

```powershell
dotnet build .\MouseMoverApp.sln
```

產生**輸出目錄僅有一個 EXE**的 Windows x64 自包含版本，須在原本的單檔設定之外加入 `IncludeNativeLibrariesForSelfExtract=true`。不加此參數時，實測旁邊仍出現 `D3DCompiler_47_cor3.dll`、`PenImc_cor3.dll`、`PresentationNative_cor3.dll`、`vcruntime140_cor3.dll`、`wpfgfx_cor3.dll`。

```powershell
dotnet publish .\MouseMoverApp.csproj -c Release -r win-x64 `
  -p:PublishSingleFile=true -p:SelfContained=true `
  -p:EnableCompressionInSingleFile=true `
  -p:IncludeNativeLibrariesForSelfExtract=true `
  -p:DebugType=None -o .\release-singlefile-complete
```

`mouseIcon.ico` 要保留於建置輸入；單檔 EXE 啟動時可能依 .NET self-extract 規則在使用者暫存位置解壓原生元件，但**交付目錄**應只有 `MouseMoverApp.exe`。如原輸出 EXE 正被執行，Windows 不允許覆寫；請輸出到新的乾淨目錄，不需終止使用者正在執行的程式。

## 6. 驗收情境

| 編號 | 操作 | 必須觀察到的結果 |
| --- | --- | --- |
| A01 | 啟動 EXE | 固定大小視窗，標題 `AZu Sanpo`、頂置、初始參考圖的排列與文字；Start、灰燈、180 min／0 sec、倒數 Off。 |
| A02 | 按 Start | 按鈕變 Stop、綠燈、文字 `Monitoring`，開始每秒更新 `Idle` 與 `In`。 |
| A03 | 監看時移動實體滑鼠或按實體鍵 | 三分鐘倒數從該活動重新開始；不產生新移動 Task。 |
| A04 | 保持三分鐘沒有真實輸入 | 進入紅燈 `Moving (Auto)`，視窗顯示並嘗試前景；游標依第 03 份 AZKi 路徑走 446 點。 |
| A05 | 走路徑中移動實體滑鼠或按鍵 | 移動儘快中斷，回綠燈 `Monitoring`；從此次輸入重新等待三分鐘，未完成的該輪不點擊。 |
| A06 | A05 後再保持三分鐘無輸入 | 恢復自動移動，重新從 AZKi 路徑起點執行。 |
| A07 | 策略完整結束 | 在終點模擬一次左鍵點擊，等待 60 秒再畫下一輪；程式自己的游標設定與點擊不造成 A05。 |
| A08 | 任何監看／移動狀態按 Stop 或 F10 | 立即取消、解除 hook，畫面回 `Not Started`／灰燈／Start，之後不會自動重啟。 |
| A09 | 勾選停止倒數並輸入正時長 | 顯示 mm:ss；更改分鐘／秒數時從完整設定時長重新計算。到期後完全停止、顯示 `Done`。 |
| A10 | 倒數設 `00:00`；或取消勾選 | 分別顯示 `set > 00:00`、`Off`；不得將閒置三分鐘誤認成停止倒數。 |
| A11 | F10 在視窗失焦時操作 | 與主按鈕一樣切換；第一次開始監看，第二次完全停止。 |
| A12 | 在同樣 DPI／字型／Windows 主題下比對畫面 | 控制項的位置、尺寸、順序、字串、指示燈顏色與 `ui-default.png` 及第 02 份表格相符。 |
| A13 | 發佈 | 建置 0 error；發佈目錄只有一個 EXE；該 EXE 能啟動並維持執行。 |

程式化測試可透過注入／直接觸發狀態變更，將最後輸入 tick 回推三分鐘來加速驗證門檻；另外仍需一次實體滑鼠或鍵盤操作來確認 hook 在目標 Windows 環境中的真實中斷效果。自動化測試須核對程式產生的左鍵事件與程式改變游標位置不被當作實體輸入。

## 7. 視覺與行為的優先順序

當規格與不同環境下的截圖存在小差異時，先核對本文件是否在同一 Windows 版本、主題、DPI 與字型測試。重建驗收順序：**功能狀態與時序 → 游標路徑和點擊 → 控制項種類與內容區幾何 → 系統主題外框**。請保留原始 UI 的簡潔原生樣式，不加新提示、設定項、額外動畫或新的操作步驟。
