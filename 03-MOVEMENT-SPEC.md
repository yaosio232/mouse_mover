# MouseMoverApp 重建規格：游標路徑與點擊

> 目前編譯設定的策略是 `AZKi`，使用者沒有切換 UI。若重建目標是「看起來與動起來都一樣」，本文件的採樣點、座標轉換、延遲及點擊順序都應照做。另一個 `Random` 策略也列於後面，以保留原始程式的可切換能力。

## 1. 架構與一次循環

原碼以 `IMouseMoveStrategy.ExecuteAsync(context, token)` 隔離策略；`Form1` 提供參數、背景執行策略、策略完成後左鍵點擊一次，再等待 60 秒。每輪重新建立 `MouseMoveStrategyContext`，但使用同一個 `Random` 物件。`AZKi` 每輪都從相同的圖形路徑開始；它**不將圖案永久留在螢幕上**，只是把游標依序移至各採樣點。

路徑定位基準是**主要螢幕的 Bounds**。沒有 PrimaryScreen 時，使用視窗所在螢幕的 Bounds。游標每一步都裁切到該 Bounds 內。開始自動移動時視窗會先嘗試置前，然後背景 Task 執行路徑。

## 2. AZKi 常數

| 名稱 | 值 | 用途 |
| --- | ---: | --- |
| `WriteScale` | `3` | 所有下述圖形局部座標的縮放倍率。 |
| `PathHeightBase` | `130` | `h=130×3=390`。 |
| `PathWidthBase` | `80` | `w=80×3=240`。 |
| `PathGapBase` | `16` | `gap=16×3=48`。 |
| `AzkiPointSplitSteps` | `1` | 每個路徑採樣點只做一次相對游標移動。 |
| `AzkiPointSplitDelayMs` | `0` | 同一採樣點內不延遲。 |
| `PathStepDelayMinMs / MaxMs` | `5 / 9` | 每個採樣點完成後，隨機等待整數 `5..9 ms`，兩端包含。 |
| K 幹與筆畫端點 | `KStemX=8`, `KTopY=8`, `KMidY=5`, `KBottomY=-5`, `KUpperEndX=48`, `KUpperEndY=15`, `KLowerEndX=50`, `KLowerEndY=-8` | 見下方 K 點位。 |
| 小寫 i 與愛心 | `ISpacingFromK=72`, `HeartSize=14`, `HeartCenterX=14`, `HeartCenterY=22` | 見下方點位。 |

以下所有座標公式以尚未縮放的 `s=WriteScale` 符號表示；開始時 `x=0, y=0, h=130s, w=80s, gap=16s, topY=y, midY=y+h/2, baseY=y+h`。在預設 `s=3` 時，`topY=0, midY=195, baseY=390`。`p` 永遠代表上一段路徑最後一個採樣點。

## 3. 採樣器，須保留端點與數值轉換

### 三次 Bézier

每個 Bézier 呼叫 `Bezier(p0,p1,p2,p3,N)`，對 **`i=0..N` 共 N+1 個點**採樣：

```text
t = i / N
u = 1 - t
X = u³ p0.X + 3u²t p1.X + 3ut² p2.X + t³ p3.X
Y = u³ p0.Y + 3u²t p1.Y + 3ut² p2.Y + t³ p3.Y
append ((int)X, (int)Y)
```

`(int)` 沿用 C# 浮點轉整數的**朝零截斷**，不要改成四捨五入。每段從 `i=0` 開始，所以接點可能重複；不要去重，否則速度與點擊時間會不同。

### 直線

每個 `Line(start,end,N)` 對 `i=0..N` 加入：

```text
X = start.X + (end.X - start.X) * i / N
Y = start.Y + (end.Y - start.Y) * i / N
```

以整數運算，除法朝零截斷。`i=0` 的起點也保留，因此各段之間仍可能重複點。

### 愛心

`Heart(center,size,N)` 對 `i=0..N` 加入：

```text
t = π - 2πi/N
x = 16 sin(t)^3
y = 13 cos(t) - 5 cos(2t) - 2 cos(3t) - cos(4t)
X = center.X + (int)(x * size / 18)
Y = center.Y - (int)(y * size / 18)
```

`X/Y` 的浮點部分同樣朝零截斷。心形的 `N=64`，總共加入 65 點。

## 4. AZKi 路徑的精確建造順序

下表 `B` 是前述 Bézier，`L` 是直線；各 `B` 的起點均為當時的 `p`，所有段完成後把 `p` 指向最後一點。第一筆先單獨加入起點。路徑實際分成 A、Z、K、小寫 i 加愛心，但各字之間也用連續線條連接；不可讓游標在字符間瞬移。

### A：初始 `x=0`

1. 加入起點 `p=(x, baseY-10s)`。
2. `B(p, (x+10s,baseY-40s), (x+18s,topY+10s), (x+26s,topY+5s), 25)`。
3. `B(p, (x+34s,topY+8s), (x+42s,midY), (x+50s,baseY-8s), 25)`。
4. `B(p, (x+44s,midY+5s), (x+28s,midY-5s), (x+52s,midY), 20)`。

### Z：先令 `x += w+gap`，預設增量 `288`

1. `B(p, (x-10s,midY-10s), (x+5s,topY+5s), (x+35s,topY+8s), 20)`。
2. `B(p, (x+50s,topY+5s), (x+40s,midY+10s), (x+8s,baseY-5s), 25)`。
3. `B(p, (x+20s,baseY), (x+45s,baseY-2s), (x+58s,baseY-8s), 20)`。

### K：再令 `x += w+gap`，預設 `x=576`

1. `B(p, (x-5s,baseY-20s), (x+2s,midY), (x+5s,topY+8s), 20)`。
2. 定義以下五個局部點：

   ```text
   kTop      = (x+8s, topY+8s)
   kMid      = (x+8s, midY+5s)
   kBottom   = (x+8s, baseY-5s)
   kUpperEnd = (x+48s, topY+15s)
   kLowerEnd = (x+50s, baseY-8s)
   ```

3. 依序 `L(p,kTop,12)`、`L(p,kBottom,28)`、`L(p,kMid,14)`、`L(p,kUpperEnd,20)`、`L(p,kMid,20)`、`L(p,kLowerEnd,22)`。每次的 `p` 都取上一條線最後一點。

### 小寫 i 與愛心：令 `x += 72s`，預設 `x=792`

1. `B(p, (x-12s,baseY-8s), (x+2s,midY+8s), (x+8s,midY-6s), 22)`。
2. `B(p, (x+10s,midY+12s), (x+10s,baseY-15s), (x+14s,baseY-6s), 22)`。
3. 設 `heartSize=14s`、`heartCenter=(x+14s,topY+22s)`、`heartBottom=(heartCenter.X,heartCenter.Y+heartSize)`。
4. `B(p, (x+20s,baseY-32s), (heartCenter.X+6s,topY+42s), heartBottom, 24)`。
5. `Heart(heartCenter,heartSize,64)`。
6. `B(p, (x+26s,topY+48s), (x+36s,midY-2s), (x+56s,midY-6s), 24)`。

按照原始迴圈包含每段端點的規則，整條路徑應有 **446 個採樣點**（含重複的接點）。每輪重新建造路徑。

## 5. 置中、移動與裁切

1. 從全部採樣點求 `minX/minY/maxX/maxY`。矩形的 `Width=maxX-minX`、`Height=maxY-minY`，不是 `+1`。
2. `offsetX=(screenBounds.Width - pathWidth)/2 - minX`，`offsetY=(screenBounds.Height - pathHeight)/2 - minY`；整數除法沿用 C# 朝零截斷。全部採樣點都加上 offset。**原碼沒有在 offset 加 `screenBounds.Left/Top`**，重建等效邏輯時應保留此點，特別是要與非零原點螢幕情境作精確比對。
3. 對每個目標點，先讀當時 `Cursor.Position`，計算目標與現位置的 `dx/dy`，呼叫相對移動。AZKi 的 step 數是 1，因此實際一次將游標設到目標點（再裁切到螢幕範圍）。
4. 一般相對移動 `MoveByDeltaAsync(dx,dy,steps,delay)`：若 steps ≤ 0 先改成 1；每一步將 `dx/steps`、`dy/steps` 累入 double accumulator，用 C# `Math.Round` 取得「至此應用過的整數位移」（預設 midpoint-to-even），扣除前一步已用位移，將差值加到當下 `Cursor.Position`。每一步開始先檢查 cancellation token。
5. 設置游標前裁切 X 至 `[Bounds.Left, Bounds.Right-1]`、Y 至 `[Bounds.Top, Bounds.Bottom-1]`。即使本步位移是零，仍會呼叫設定游標位置。
6. 每個 AZKi 採樣點之後等待隨機 `5..9 ms`；若中途取消則立即離開。策略完整結束才執行一次左鍵點擊。

## 6. 保留的 Random 策略

切換原始 `CURRENT_MOVE_MODE` 編譯常數為 `Random` 才會使用；UI 無模式選擇器。每個策略循環做 10 次隨機相對移動：X 與 Y 各自獨立取整數 `-450..450`（端點包含），每次拆為 50 步，每步延遲 4 ms，之後隨機暫停 `200..1800 ms`（端點包含）。10 次全部完成後，由 Form 做同一種左鍵點擊與 60 秒等待。相對移動的 rounding、螢幕裁切及取消規則與上一節完全相同。
