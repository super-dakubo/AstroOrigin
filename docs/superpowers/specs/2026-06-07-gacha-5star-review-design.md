# 5★ 出卡回顾面板 — 替换欧非曲线

## 背景

当前抽卡页面板展示「欧非曲线」（LuckChart），X 轴为第 N 次 5★，Y 轴为距上一次 5★ 的间隔抽数。该图表信息量低，用户反馈「什么信息也没有」。

## 目标

将 LuckChart 替换为「5★ 出卡回顾」面板，以紧凑的卡片列表展示每次 5★ 出货的核心信息：花费抽数、欧非判定、卡池类型。

## 设计

### 默认态

每次 5★ 一条水平行，从左到右：

- **名称** + ★★★★★ 标记
- **花费抽数**（大字突出）
- **欧非徽章**（彩色标签）
- **日期**（右侧）

三条边框颜色标识状态：

- 🟢 绿色 — 欧（小保底没歪）
- 🔴 红色 — 非（歪了 / 满保底）
- 🟡 橙色 — 大保底出货
- ⚪ 灰色 — 无 50/50（常驻/新手池）

### 悬浮态

鼠标悬停弹出深色 tooltip，展示详情：

- 卡池名称
- 保底类型（小保底未歪 / 小保底歪了 / 大保底）
- 花费抽数（大保底情况显示合计：上次歪的抽数 + 本次抽数）
- 欧非评级文字（如「SSS 级欧皇」「究极非酋」）

### 顶部统计摘要

面板标题行右侧展示：

- 平均抽数/5★
- 小保底胜率
- 最长保底记录

## 核心逻辑

### 大保底判定

遍历 5★ 记录时，需追踪上下文判定保底类型：

```
遍历 records（按时间 ASC，即从旧到新）：
  如果 record 是限定池（角色/武器/光锥活动 or 集录祈愿）：
    如果 lastWasLimitedAndLost == true：
      → 大保底（徽章: 大保底）
      lastWasLimitedAndLost = false
    否则：
      → 小保底
      如果 isWon == false：
        lastWasLimitedAndLost = true（下一条限定池是大保底）
      否则：
        lastWasLimitedAndLost = false
  如果 record 是常驻/新手池：
    → 无 50/50（徽章: -）
    lastWasLimitedAndLost 不变（常驻记录不影响保底继承）
```

### 欧非评级

基于花费抽数分级（仅适用有 50/50 的卡池）：

| 花费抽数 | 评级          |
| -------- | ------------- |
| 1-10     | ⚡ SSS 级欧皇 |
| 11-30    | ✨ 欧！       |
| 31-55    | ✅ 不错       |
| 56-75    | 正常范围      |
| 76-85    | 💀 非         |
| 86+      | 💀 究极非酋   |

大保底情况额外显示合计花费（歪的抽数 + 大保底抽数）。

### 数据来源

复用 `get_gacha_chart_records` Tauri 命令（已存在，返回全量记录不分页），无需新增后端命令。

### 歪 / 不歪判定

当前数据库 `is_won` 字段在 OCR 和 API 导入时均硬编码为 `true`，不反映实际 50/50 结果。需在遍历时通过以下规则判定：

**规则：** 在限定池（角色活动/武器活动/光锥活动/集录）中，如果出现的 5★ 角色名是「常驻角色」，则判定为歪（`isWon = false`）。否则为没歪（`isWon = true`）。

常驻角色名单（不随版本变化）：

星穹铁道常驻 5★ 角色：白露、布洛妮娅、姬子、杰帕德、克拉拉、瓦尔特、彦卿
（星铁道常驻光锥不计，活动光锥池无歪概念 / 75-25）

原神常驻 5★ 角色：迪卢克、琴、刻晴、莫娜、七七、提纳里、迪希雅
原神常驻 5★ 武器：天空系列、风鹰剑、和璞鸢、阿莫斯之弓、四风原典、狼的末路、贯虹之槊

**算法流程：**

```
遍历 5★ records（按时间 ASC，即从旧到新）：
  if record 是常驻/新手/集录池：
    → 无 50/50
    lastLimitedLost 保持
  else if record.bannerType 是限定池（角色/武器/光锥活动）:
    if record.itemName 在常驻名单中:
      → 判定为歪（isWon = false）
      lastLimitedLost = true（下一条限定池是大保底）
    else:
      → 判定为没歪（isWon = true）
      if lastLimitedLost:
        → 大保底出货
        lastLimitedLost = false
      else:
        → 小保底没歪
        lastLimitedLost = false
```

### 计算顺序与展示顺序

- **保底判定使用 ASC**（从旧到新，因为大保底依赖前一跳的状态）
- **展示使用 DESC**（最新在前，符合用户习惯）
- 组件内部：先 ASC 遍历计算保底类型 → 再 DESC 排序传给渲染

## 数据结构

```typescript
interface FiveStarReviewItem {
  id: number
  itemName: string
  starRating: number
  pulls: number // 距上一次 5★ 的间隔抽数
  bannerType: string
  recordDate: string
  isWon: boolean | null // null 表示无 50/50
  isGuaranteed: boolean // 是否大保底
  lostToName?: string // 歪给了谁（前一次记录的名称）
  rating: string // 评级文字
}
```

## 后端改动

无。`get_gacha_chart_records` 已存在，返回全量 `GachaRecord[]`。

## 前端改动

### 新增或修改的文件

1. `frontend/components/FiveStarReview.tsx`（新）— 5★ 出卡回顾组件
2. `frontend/pages/Gacha.tsx` — 将 `<LuckChart>` 替换为 `<FiveStarReview>`
3. `frontend/components/LuckChart.tsx` — 可移除或保留 unused 状态

### FiveStarReview 组件职责

- 消费全量 GachaRecord[]
- 过滤 5★ 记录
- 按时间 ASC 排序后遍历，计算每条的保底类型和评级
- 渲染卡片列表 + tooltip + 顶部统计

## 数据流

```
get_gacha_chart_records → allGachaRecords → FiveStarReview
                                              ↓
                                      过滤 5★ → 排序 ASC → 遍历判定保底类型 → 计算评级 → 渲染
```

## 不包含

- 双黄/多黄识别（需要更细粒度的时间聚类，后续可加）
- 武器池定轨/命定值统计
- 导出/分享功能
