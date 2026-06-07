# 5★ 出卡回顾面板 — 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 替换抽卡页面板的 LuckChart 为 FiveStarReview 卡片列表组件

**Architecture:** 纯前端改动，复用后端 `get_gacha_chart_records` 返回的全量记录。FiveStarReview 组件消费 GachaRecord[]，按 ASC 遍历计算保底类型，按 DESC 渲染卡片列表。常驻角色名单做启发式歪判定。

**Tech Stack:** React 18, TypeScript, Tailwind CSS

---

## 文件结构

- **Create:** `frontend/components/FiveStarReview.tsx` — 5★ 出卡回顾组件（~200 行）
- **Modify:** `frontend/pages/Gacha.tsx` — 将 `<LuckChart records={chartData} />` 替换为 `<FiveStarReview records={allGachaRecords} />`
- **Unused (keep):** `frontend/components/LuckChart.tsx` — 不再被引用，保留未删除

### Task 1: 创建 FiveStarReview 组件

**Files:**
- Create: `frontend/components/FiveStarReview.tsx`

- [ ] **Step 1: 创建组件框架，定义常驻名单和类型**

```typescript
import { useMemo } from 'react'
import type { GachaRecord } from '../lib/types'

interface FiveStarReviewItem {
  id: number
  itemName: string
  starRating: number
  pulls: number
  bannerType: string
  recordDate: string
  isWon: boolean | null
  isGuaranteed: boolean
  rating: string
}

interface FiveStarReviewProps {
  records: GachaRecord[]
}

// 星穹铁道常驻 5★ 角色
const SR_STANDARD = new Set([
  '白露', '布洛妮娅', '姬子', '杰帕德', '克拉拉', '瓦尔特', '彦卿'
])

// 原神常驻 5★ 角色
const GI_STANDARD = new Set([
  '迪卢克', '琴', '刻晴', '莫娜', '七七', '提纳里', '迪希雅',
  // 常驻武器（天空系列 + 其他）
  '天空之刃', '天空之卷', '天空之翼', '天空之傲', '天空之脊',
  '风鹰剑', '和璞鸢', '阿莫斯之弓', '四风原典', '狼的末路', '贯虹之槊',
  '斫峰之刃', '无工之剑', '尘世之锁', '不灭月华',
])

// 判断是否限定池（有 50/50）
function isLimitedBanner(bannerType: string): boolean {
  return bannerType.includes('角色活动') || bannerType.includes('武器活动')
    || bannerType.includes('光锥活动') || bannerType.includes('集录')
}

// 判断是否歪：限定池里出了常驻角色
function isLost(gameKind: string, itemName: string, bannerType: string): boolean | null {
  if (!isLimitedBanner(bannerType)) return null
  const pool = gameKind === 'starrail' ? SR_STANDARD : GI_STANDARD
  return pool.has(itemName) ? true : false
}

// 欧非评级
function getRating(pulls: number, isGuaranteed: boolean): string {
  if (isGuaranteed) return '大保底'
  if (pulls <= 10) return '⚡ SSS 级欧皇'
  if (pulls <= 30) return '✨ 欧！'
  if (pulls <= 55) return '✅ 不错'
  if (pulls <= 75) return '正常范围'
  if (pulls <= 85) return '💀 非'
  return '💀 究极非酋'
}

export function FiveStarReview({ records }: FiveStarReviewProps) {
  // ... 计算逻辑 + 渲染
}
```

- [ ] **Step 2: 实现计算逻辑（useMemo）**

```typescript
const reviewItems = useMemo((): FiveStarReviewItem[] => {
  // 过滤 5★ 并按时间 ASC 排序
  const fiveStars = records
    .filter((r) => r.starRating === 5)
    .sort((a, b) => a.id - b.id) // ASC

  let lastLimitedLost = false
  let lostPullCount = 0
  let lastLimitedPullCount = 0
  const items: FiveStarReviewItem[] = []

  for (let i = 0; i < fiveStars.length; i++) {
    const r = fiveStars[i]
    const bannerType = r.bannerType || ''

    // 计算抽数间隔（距前一条记录的 id 差）
    const prevId = i > 0 ? fiveStars[i - 1].id : 0
    const pulls = records.filter((rec) => rec.id > prevId && rec.id <= r.id).length

    if (!isLimitedBanner(bannerType)) {
      // 常驻/新手/集录 无 50/50
      items.push({
        id: r.id,
        itemName: r.itemName,
        starRating: 5,
        pulls,
        bannerType,
        recordDate: r.recordDate,
        isWon: null,
        isGuaranteed: false,
        rating: '-',
      })
      lostPullCount = 0
      continue
    }

    // 限定池 — 判定歪不歪
    const lost = isLost(records[0]?.gameKind, r.itemName, bannerType)

    if (lost === true) {
      // 歪了 → 下一条标记大保底
      items.push({
        id: r.id,
        itemName: r.itemName,
        starRating: 5,
        pulls,
        bannerType,
        recordDate: r.recordDate,
        isWon: false,
        isGuaranteed: false,
        rating: getRating(pulls, false),
      })
      lastLimitedLost = true
      lostPullCount = pulls
    } else if (lastLimitedLost) {
      // 大保底出货
      items.push({
        id: r.id,
        itemName: r.itemName,
        starRating: 5,
        pulls,
        bannerType,
        recordDate: r.recordDate,
        isWon: true,
        isGuaranteed: true,
        rating: getRating(pulls, true),
      })
      lastLimitedLost = false
      lostPullCount = 0
    } else {
      // 小保底没歪
      items.push({
        id: r.id,
        itemName: r.itemName,
        starRating: 5,
        pulls,
        bannerType,
        recordDate: r.recordDate,
        isWon: true,
        isGuaranteed: false,
        rating: getRating(pulls, false),
      })
      lastLimitedLost = false
      lostPullCount = 0
    }
  }

  // 返回 DESC 顺序（最新在前）
  return items.reverse()
}, [records])
```

- [ ] **Step 3: 实现顶部统计摘要**

```typescript
// 在组件内，reviewItems 之后计算统计
const stats = useMemo(() => {
  const limitedItems = reviewItems.filter((i) => isLimitedBanner(i.bannerType))
  const totalPulls = reviewItems.reduce((s, i) => s + i.pulls, 0)
  const limitedTotal = limitedItems.length
  const wonCount = limitedItems.filter((i) => i.isWon === true && !i.isGuaranteed).length
  const totalNormal = limitedItems.filter((i) => !i.isGuaranteed).length
  const winRate = totalNormal > 0 ? Math.round((wonCount / totalNormal) * 100) : 0
  const longestPulls = limitedItems.reduce((max, i) => Math.max(max, i.pulls), 0)

  return {
    avg: limitedTotal > 0 ? (totalPulls / limitedTotal).toFixed(1) : '--',
    winRate,
    longest: longestPulls || '--',
  }
}, [reviewItems])

// 渲染：
{
  /* <div className="flex gap-4 text-xs text-gray-500">
    <span>平均 {stats.avg} 抽/5★</span>
    <span>小保底率 {stats.winRate}%</span>
    <span>最长 {stats.longest} 抽</span>
  </div> */
}
```

- [ ] **Step 4: 实现渲染（卡片列表 + hover tooltip）**

```jsx
{reviewItems.length === 0 ? (
  <div className="bg-white rounded-xl border border-gray-200 p-8 text-center text-gray-400 text-sm">
    暂无 5★ 出货记录
  </div>
) : (
  <div className="space-y-2">
    {reviewItems.map((item) => {
      const isWonColor = item.isWon === false ? 'border-l-red-500' :
        item.isGuaranteed ? 'border-l-amber-500' :
        item.isWon === true ? 'border-l-emerald-500' :
        'border-l-gray-300'

      return (
        <div
          key={item.id}
          className={`bg-white rounded-xl border border-l-4 border-gray-200 ${isWonColor} p-3 relative group cursor-default transition-shadow hover:shadow-md`}
        >
          {/* 默认态：一行 */}
          <div className="flex items-center gap-3">
            <span className="text-sm font-semibold text-gray-900 min-w-[72px]">
              {item.itemName}
              <span className="text-amber-500 ml-1">★★★★★</span>
            </span>
            <span className="text-xl font-bold text-gray-900 min-w-[52px] text-center">
              {item.pulls}<span className="text-xs font-normal text-gray-400 ml-0.5">抽</span>
            </span>

            {/* 徽章 */}
            {item.isWon === false ? (
              <span className="text-xs font-medium px-2 py-0.5 rounded bg-red-50 text-red-500 min-w-[44px] text-center">歪 ✗</span>
            ) : item.isGuaranteed ? (
              <span className="text-xs font-medium px-2 py-0.5 rounded bg-amber-50 text-amber-600 min-w-[52px] text-center">大保底</span>
            ) : item.isWon === true ? (
              <span className="text-xs font-medium px-2 py-0.5 rounded bg-green-50 text-green-600 min-w-[44px] text-center">欧 ✓</span>
            ) : (
              <span className="text-xs font-medium px-2 py-0.5 rounded bg-gray-50 text-gray-400 min-w-[28px] text-center">-</span>
            )}

            <span className="text-xs text-gray-400 ml-auto">{item.recordDate}</span>
          </div>

          {/* 悬浮 tooltip */}
          <div className="hidden group-hover:block absolute top-full left-0 right-0 mt-2 z-20 bg-slate-800 text-slate-100 rounded-lg p-3 text-xs shadow-lg before:content-[''] before:absolute before:-top-1.5 before:left-8 before:w-3 before:h-3 before:bg-slate-800 before:rotate-45">
            <div className="flex justify-between py-1"><span className="text-slate-400">卡池</span><span className="font-medium">{item.bannerType}</span></div>
            <div className="flex justify-between py-1"><span className="text-slate-400">保底类型</span><span className="font-medium">{item.isGuaranteed ? '大保底' : item.isWon === null ? '无 50/50' : '小保底'}</span></div>
            <div className="flex justify-between py-1"><span className="text-slate-400">花费</span><span className="font-medium">{item.pulls} 抽</span></div>
            {item.isWon === false && (
              <div className="flex justify-between py-1"><span className="text-slate-400">下一条</span><span className="font-medium text-amber-400">→ 大保底继承</span></div>
            )}
            {item.isGuaranteed && (
              <div className="border-t border-slate-600 my-1" />
            )}
            <div className="text-center pt-1 text-sm font-bold"
              style={{
                color: item.isWon === false ? '#f87171' :
                  item.isGuaranteed ? '#fbbf24' :
                  item.isWon === true ? '#34d399' : '#94a3b8'
              }}
            >
              {item.rating}
            </div>
          </div>
        </div>
      )
    })}
  </div>
)}
```

- [ ] **Step 5: 检查 TypeScript 编译**

```bash
cd d:/code/AstroOrigin && npx tsc --noEmit
```

Expected: 无编译错误

### Task 2: 将 FiveStarReview 接入 Gacha.tsx

**Files:**
- Modify: `frontend/pages/Gacha.tsx`

- [ ] **Step 1: 替换 import 和组件引用**

移除:
```tsx
import { LuckChart } from '../components/LuckChart'
```

添加:
```tsx
import { FiveStarReview } from '../components/FiveStarReview'
```

将渲染中的:
```tsx
<LuckChart records={chartData} />
```

替换为:
```tsx
<FiveStarReview records={allGachaRecords} />
```

`allGachaRecords` 已存在（之前为 LuckChart 加的 `get_gacha_chart_records` 数据）。

- [ ] **Step 2: 清理不再需要的 `chartData` 和 `chartSourceRecords`**

移除以下代码（不再需要，FiveStarReview 自己处理记录过滤和计算）：
```tsx
const has5050 = bannerTab === '全部' || ...
const chartData = useMemo(() => ..., [...])
```

但保留 `allGachaRecords`（仍为 FiveStarReview 使用）和 `chartSourceRecords` 如果其他地方有引用。

- [ ] **Step 3: 检查并清除 LuckChart 相关引用**

确认 Gacha.tsx 不再引用 LuckChart：
```bash
grep -n "LuckChart\|chartData\|chartSourceRecords\|has5050" frontend/pages/Gacha.tsx
```

移除所有未使用的变量，确保 `npx tsc --noEmit` 无错误。

- [ ] **Step 4: 提交**

```bash
git add frontend/components/FiveStarReview.tsx frontend/pages/Gacha.tsx
git commit -m "feat: replace LuckChart with 5★ review panel

- New FiveStarReview component with pity type detection
- Per-record 5★ pull count, won/lost badge, hover details
- Standard character heuristic for 50/50 loss detection
- Top stats: avg pulls, win rate, longest streak"
```
