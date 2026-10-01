# v1.0.0 驗證記錄

日期：2026-10-01。Windows x64，.NET SDK 8.0.409。

## 已執行

- `dotnet build -c Release`：0 警告、0 錯誤。
- `dotnet run --project tests/MouseMoverApp.Checks -c Release`：六組核心邏輯檢查、一組 WinForms／Win32 整合檢查。
- 閒置門檻之前不移動、滿三分鐘才移動、使用者輸入後再次等待三分鐘。
- 停止倒數到期優先於移動，完全停止；輸入不中斷倒數，Start 重設倒數，零／未勾選／手動停止正確。
- AZKi 路徑 446 點、包含重複接點、固定端點、置中保留不加螢幕原點的原版行為。
- 相對位移累積 rounding-to-even、螢幕邊界裁切、已取消 token 不操作游標。
- 真實 Win32 hook 安裝和解除；滑鼠 callback 依注入 bit 0 過濾，鍵盤依正確的 `LLKHF_INJECTED (0x10)` 過濾；實體延伸鍵 metadata 恢復 Monitoring。
- 真實模擬左鍵點擊只送往程式自己的非互動狀態標籤；hook 觀察到注入 down/up 旗標。
- F10 在另一個測試視窗取得焦點时，第一次開始監看，第二次完全停止。
- 原生控制項 bitmap 與 `ui-default.png` 比對：四列排列、數值、文字與視窗尺寸相符；焦點框／Windows 外框依環境繪製。
- `scripts/publish.ps1` 驗證交付目錄僅一個 `MouseMoverApp.exe`（71,713,927 bytes），SHA256 另存上一層。
- 最終 EXE SHA256：`6c3829b1ac608c0d1cd325561414b4491e2b91d1bb28ce9e63e138808bc49f37`。
- 獨立程式碼審查已確認鍵盤旗標修正，沒有重要未處理問題。
- 發佈 EXE 實際啟動三秒，視窗標題 `AZu Sanpo`、持續執行，正常關閉。
- 原始碼與待提交文件掃描未發現 Token、私鑰、密碼或使用者個人設定；建置產物與憑證檔由 `.gitignore` 排除。

## 實機驗收範圍

完整測試偵測到測試期間存在非注入的滑鼠活動，因此該次不宣稱在完全靜止桌面上驗證整段移動。自動檢查已分別驗證注入旗標、callback 狀態轉移與純邏輯取消。以下保留給使用者在目標機器確認：

1. Start 後實際閒置三分鐘，確認 AZKi 路徑、完整後單次點擊、60 秒後重複。
2. 路徑中操作實體滑鼠及鍵盤，確認立即中斷、未完成輪不點擊，三分鐘後恢復。
3. 靜止桌面上確認整輪程式游標設定／點擊不重設 Idle。
4. 若需像素完全一致，固定 Windows 佈景、字型、DPI 後比對參考圖片。

技術差異：採 .NET 8 自包含發佈；原始 icon 未提供，使用重製藍色滑鼠圖示；修正歷史規格的鍵盤旗標錯誤（bit 0 是延伸鍵，bit 4 才是注入），詳見 [Microsoft 文件](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-kbdllhookstruct)。
