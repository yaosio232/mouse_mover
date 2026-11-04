# 1 秒重現自動移動問題

目前的使用者設定是 `mode=random`、`transparent=false`。在另一台 Windows 電腦先用「設定」選擇隨機滑動，再於 PowerShell 執行：

```powershell
$env:AZUSANPO_TEST_IDLE_SECONDS = '1'
cargo +stable-x86_64-pc-windows-gnu run --locked
```

按 `F10` 開始後，讓滑鼠保持靜止。除錯版會在閒置 1 秒後執行與正式版相同的移動程式碼；若出現錯誤，記下對話框的完整文字。此環境變數只在除錯版生效，正式版仍使用設定選單的閒置分鐘數。

這個分支只提供快速重現入口，尚未修正滑鼠移動錯誤或更新 SETTINGS 圖示。使用者指定的齒輪輪廓存於 [`settings-reference.svg`](../settings-reference.svg)，供下一台電腦實作時對照。
