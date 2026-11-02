Mouse Mover App

全 Rust 滑鼠散步工具，使用原生 Win32 控制項，不需要 .NET 或額外 GUI 執行環境。

下載與操作

從 https://github.com/yaosio232/mouse_mover/releases 下載 MouseMoverApp.exe，直接執行。支援 Windows x64。

- 按 Start (F10) 或全域 F10，開始監看真實滑鼠及鍵盤輸入。
- 閒置三分鐘後，游標會開始移動，完成後左鍵點擊一次，等待 60 秒再重複。
- 鍵盤輸入或滑鼠移動時會取消移動，直到再度閒置三分鐘。
- 按 Stop 或 F10 完全停止。

額外功能

- Stop after 是獨立的停止倒數；勾選／修改時長立即重新計時，Start 時也重新計時，到期顯示 Stop done 並完全停止。預設未勾選，3 小時／0 分鐘／0 秒。
- 棋盤格圖示可切換透明背景；文字去背，按鈕與輸入框保持不透明。兩種模式下，整個按鈕與勾選框區域皆可點擊。
- 關閉視窗即停止；沒有背景服務、設定儲存或網路功能。

建置與發佈

需要 Windows、Rust stable（x86_64-pc-windows-gnu），以及 PATH 中相容的 x64 MinGW-w64 工具鏈（GCC、dlltool、windres）：

cargo build --release --locked
cargo test --locked
./scripts/publish.ps1

發佈目錄 artifacts/release 含 MouseMoverApp.exe、README_EN.txt 與 README_ZH-TW.txt。
