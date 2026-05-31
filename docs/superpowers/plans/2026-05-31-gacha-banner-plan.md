# Gacha Banner Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split gacha records display by banner type (角色活动/光锥活动/常驻), add filtering/sorting/pagination, make is_won toggleable.

**Architecture:** Backend enriches `get_gacha_records` with banner/star/sort query params and `get_gacha_stats` with per-banner breakdown. Frontend adds banner tabs, filter bar, and clickable is_won toggle to RecordTable.

**Tech Stack:** Rust (rusqlite + serde) backend, React/TypeScript (Zustand + react-query) frontend, Tailwind CSS grid layout.

**Already done:** DB migration for `banner_type`, GachaRecord struct + upsert saves banner_type, update_gacha_record supports banner_type, RecordTable has pagination shell.

---

### Task 1: Backend — get_gacha_records pagination/sorting/filtering

**Files:**
- Modify: `src-tauri/src/commands/gacha.rs:82-132`

- [ ] **Step 1: Update get_gacha_records signature and defaults**

Change function signature to accept banner filter, star filter, sort params. Default page_size to 20.

```rust
#[tauri::command]
pub async fn get_gacha_records(
    pool: tauri::State<'_, DbPool>,
    game_kind: String,
    page: Option<i64>,
    page_size: Option<i64>,
    banner: Option<String>,         // "角色活动" / "光锥活动" / "武器活动" / "常驻" / null=全部
    star_filter: Option<i32>,       // null=全部, 5/4/3
    sort_by: Option<String>,        // "date" / "star"
    sort_order: Option<String>,     // "asc" / "desc"
) -> TauriResult<GachaRecordsResponse> {
    let p = page.unwrap_or(1).max(1);
    let ps = page_size.unwrap_or(20).clamp(1, 200);  // ← 默认20
    let offset = (p - 1) * ps;
```

- [ ] **Step 2: Build dynamic WHERE clause**

Replace the static `WHERE game_kind = ?` with dynamic SQL that adds banner filter and star filter.

```rust
let pool = pool.inner().clone();

tokio::task::spawn_blocking(move || {
    let conn = pool.get().context("Failed to get DB connection")?;

    // 构建条件
    let mut conditions = vec!["game_kind = ?1".to_string()];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(game_kind.clone())];

    // banner 筛选（模糊匹配，如传入"角色活动"匹配"角色活动跃迁"和"角色活动祈愿"）
    if let Some(ref b) = banner {
        if !b.is_empty() && b != "全部" {
            let param_idx = params.len() + 1;
            conditions.push(format!("banner_type LIKE ?{param_idx}"));
            params.push(Box::new(format!("%{}%", b)));
        }
    }

    // 星级筛选
    if let Some(sf) = star_filter {
        if sf > 0 {
            let param_idx = params.len() + 1;
            conditions.push(format!("star_rating = ?{param_idx}"));
            params.push(Box::new(sf));
        }
    }

    // 排序
    let order_clause = match (sort_by.as_deref(), sort_order.as_deref()) {
        (Some("star"), Some("asc")) => "star_rating ASC, record_date DESC".to_string(),
        (Some("star"), _) => "star_rating DESC, record_date DESC".to_string(),
        (_, Some("asc")) => "record_date ASC, id ASC".to_string(),
        _ => "record_date DESC, id DESC".to_string(),
    };

    // COUNT
    let count_sql = format!("SELECT COUNT(*) FROM gacha_records WHERE {}", conditions.join(" AND "));
    let total: i64 = conn.query_row(&count_sql, rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| row.get(0))?;

    // SELECT with LIMIT/OFFSET
    let query_sql = format!(
        "SELECT id, game_kind, item_name, item_type, star_rating, record_date, is_won, banner_type
         FROM gacha_records
         WHERE {}
         ORDER BY {}
         LIMIT ?{} OFFSET ?{}",
        conditions.join(" AND "),
        order_clause,
        params.len() + 1,
        params.len() + 2,
    );
```

Note: Since rusqlite's params_from_iter can be tricky with dynamic params, a simpler approach is to use positional params with a single params array:

```rust
    // Collect all params into a Vec
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    param_values.push(Box::new(game_kind.clone()));

    if let Some(ref b) = banner {
        if !b.is_empty() && b != "全部" {
            conditions.push(format!("banner_type LIKE ?{}", param_values.len() + 1));
            param_values.push(Box::new(format!("%{}%", b)));
        }
    }
    if let Some(sf) = star_filter {
        if sf > 0 {
            conditions.push(format!("star_rating = ?{}", param_values.len() + 1));
            param_values.push(Box::new(sf));
        }
    }

    let where_clause = conditions.join(" AND ");
    let count_sql = format!("SELECT COUNT(*) FROM gacha_records WHERE {}", where_clause);
    let total: i64 = conn.query_row(
        &count_sql,
        rusqlite::params_from_iter(param_values.iter().map(|p| p.as_ref())),
        |row| row.get(0),
    )?;

    param_values.push(Box::new(ps));  // LIMIT
    param_values.push(Box::new(offset));  // OFFSET

    let query_sql = format!(
        "SELECT id, game_kind, item_name, item_type, star_rating, record_date, is_won, banner_type
         FROM gacha_records
         WHERE {}
         ORDER BY {}
         LIMIT ?{} OFFSET ?{}",
        where_clause,
        order_clause,
        param_values.len() - 1,
        param_values.len(),
    );

    let mut stmt = conn.prepare(&query_sql)?;
    let records = stmt
        .query_map(
            rusqlite::params_from_iter(param_values.iter().map(|p| p.as_ref())),
            |row| {
                Ok(GachaRecord {
                    id: row.get(0)?,
                    game_kind: row.get(1)?,
                    item_name: row.get(2)?,
                    item_type: row.get(3)?,
                    star_rating: row.get(4)?,
                    record_date: row.get(5)?,
                    is_won: row.get(6)?,
                    banner_type: row.get(7)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to collect records")?;

    Ok(GachaRecordsResponse { records, total })
})
```

- [ ] **Step 3: Compile check**

```bash
cargo check --manifest-path src-tauri/Cargo.toml 2>&1
```
Expected: No errors (only pre-existing warnings).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/gacha.rs
git commit -m "feat: add banner/star/sort params to get_gacha_records, default page_size 20"
```

---

### Task 2: Backend — get_gacha_stats with per-banner breakdown

**Files:**
- Modify: `src-tauri/src/commands/gacha.rs`

- [ ] **Step 1: Add BannerStats struct**

Add after `GachaStats`:

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BannerStats {
    pub banner_type: String,
    pub total_pulls: i64,
    pub five_star_count: i64,
    pub lost_count: i64,
    pub current_pity: i32,
    pub avg_pulls_per_five_star: f64,
}
```

Add `by_banner` field to `GachaStats`:

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaStats {
    pub total_pulls: i64,
    pub five_star_count: i64,
    pub lost_count: i64,
    pub current_pity: i32,
    pub avg_pulls_per_five_star: f64,
    pub by_banner: Vec<BannerStats>,  // 新增
}
```

- [ ] **Step 2: Query per-banner stats**

After the existing stats queries, add:

```rust
// 按 banner_type 分组统计
let mut by_banner = Vec::new();
let mut banner_stmt = conn.prepare(
    "SELECT banner_type, COUNT(*), SUM(CASE WHEN star_rating = 5 THEN 1 ELSE 0 END),
            SUM(CASE WHEN star_rating = 5 AND is_won = 0 THEN 1 ELSE 0 END)
     FROM gacha_records
     WHERE game_kind = ?
     GROUP BY banner_type
     ORDER BY banner_type"
)?;

let banner_rows = banner_stmt.query_map(rusqlite::params![game_kind], |row| {
    let banner_type: String = row.get(0)?;
    let total: i64 = row.get(1)?;
    let five_star: i64 = row.get(2)?;
    let lost: i64 = row.get(3)?;
    Ok((banner_type, total, five_star, lost))
})?
.collect::<Result<Vec<_>, _>>()
.context("Failed to collect banner stats")?;

for (banner_type, total, five_star, lost) in &banner_rows {
    // 计算每个卡池的当前保底
    let latest_five_id: Option<i64> = conn.query_row(
        "SELECT MAX(id) FROM gacha_records WHERE game_kind = ? AND star_rating = 5 AND banner_type = ?",
        rusqlite::params![game_kind, banner_type],
        |row| row.get(0),
    ).ok();

    let pity: i32 = if let Some(max_id) = latest_five_id {
        conn.query_row(
            "SELECT COUNT(*) FROM gacha_records WHERE game_kind = ? AND id > ? AND banner_type = ?",
            rusqlite::params![game_kind, max_id, banner_type],
            |row| row.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(*) FROM gacha_records WHERE game_kind = ? AND banner_type = ?",
            rusqlite::params![game_kind, banner_type],
            |row| row.get(0),
        )?
    };

    let avg = if *five_star > 0 { *total as f64 / *five_star as f64 } else { 0.0 };

    by_banner.push(BannerStats {
        banner_type: banner_type.clone(),
        total_pulls: *total,
        five_star_count: *five_star,
        lost_count: *lost,
        current_pity: pity,
        avg_pulls_per_five_star: avg,
    });
}
```

Add `by_banner` to the returned GachaStats:

```rust
Ok(GachaStats {
    total_pulls: total,
    five_star_count: five_star,
    lost_count: lost,
    current_pity,
    avg_pulls_per_five_star: avg_pulls,
    by_banner,  // 新增
})
```

- [ ] **Step 3: Compile check**

```bash
cargo check --manifest-path src-tauri/Cargo.toml 2>&1
```
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/gacha.rs
git commit -m "feat: add per-banner stats to get_gacha_stats"
```

---

### Task 3: Frontend — GachaRecord interface and RecordTable banner_type column

**Files:**
- Modify: `frontend/pages/Gacha.tsx`
- Modify: `frontend/components/RecordTable.tsx`

- [ ] **Step 1: Add bannerType to GachaRecord interface in both files**

In `Gacha.tsx`:
```typescript
interface GachaRecord {
  id: number
  gameKind: string
  itemName: string
  itemType: string
  bannerType: string   // 新增
  starRating: number
  recordDate: string
  isWon: boolean
}
```

In `RecordTable.tsx`:
```typescript
interface GachaRecord {
  id: number
  gameKind: string
  itemName: string
  itemType: string
  bannerType: string   // 新增
  starRating: number
  recordDate: string
  isWon: boolean
}
```

- [ ] **Step 2: Add bannerType to RecordTable display**

Add banner type column to the grid. Change grid from `grid-cols-[1.5fr_0.8fr_2.5fr_70px_70px_40px]` (6 columns) to `grid-cols-[1.5fr_0.8fr_0.8fr_2fr_60px_60px_30px]` (7 columns).

Update headers in `RecordTable.tsx`:
```tsx
// Header row (in .grid container):
<div className="grid grid-cols-[1.5fr_0.8fr_0.8fr_2fr_60px_60px_30px] gap-2 px-4 py-2.5 bg-gray-50 text-xs font-medium text-gray-400 items-center">
  <span>日期</span>
  <span>种类</span>
  <span>卡池</span>
  <span>物品</span>
  <span>星级</span>
  <span>结果</span>
  <span />
</div>
```

Display row (add after itemType column):
```tsx
// Card pool tag with color
<span>
  {r.bannerType ? (
    <span className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${
      r.bannerType.includes('角色')
        ? 'bg-blue-50 text-blue-600'
        : r.bannerType.includes('光锥') || r.bannerType.includes('武器')
          ? 'bg-purple-50 text-purple-600'
          : 'bg-gray-50 text-gray-500'
    }`}>
      {r.bannerType.replace('跃迁', '').replace('祈愿', '')}
    </span>
  ) : ''}
</span>
```

Update all grid layouts in both view and edit modes to match the 7-column structure.

- [ ] **Step 3: TypeScript check**

```bash
cd frontend && npx tsc --noEmit 2>&1
```
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add frontend/pages/Gacha.tsx frontend/components/RecordTable.tsx
git commit -m "feat: add banner_type column to RecordTable"
```

---

### Task 4: Frontend — Clickable is_won toggle

**Files:**
- Modify: `frontend/components/RecordTable.tsx`
- Modify: `frontend/pages/Gacha.tsx`

- [ ] **Step 1: Add onToggleWon prop to RecordTable**

```typescript
interface RecordTableProps {
  records: GachaRecord[]
  onDelete?: (id: number) => void
  onSave?: (
    id: number,
    data: { itemName: string; itemType: string; starRating: number; recordDate: string; isWon: boolean; bannerType: string }
  ) => void
  onToggleWon?: (id: number, isWon: boolean) => void  // 新增：一键切换
  page: number
  total: number
  pageSize: number
  onPageChange: (page: number) => void
}
```

- [ ] **Step 2: Replace static isWon display with clickable toggle**

In the record display row, replace the isWon text with a clickable button:

```tsx
// 结果列 — 只有5★可以切换歪/不歪
<span>
  {r.starRating === 5 ? (
    <button
      onClick={() => onToggleWon?.(r.id, !r.isWon)}
      className={`px-2 py-0.5 rounded text-xs font-medium cursor-pointer transition-colors ${
        r.isWon
          ? 'bg-green-50 text-green-600 hover:bg-green-100'
          : 'bg-red-50 text-red-500 hover:bg-red-100'
      }`}
    >
      {r.isWon ? '欧 ✓' : '歪了'}
    </button>
  ) : (
    <span className="text-gray-300 text-xs">-</span>
  )}
</span>
```

- [ ] **Step 3: Add onToggleWon handler in Gacha.tsx**

Wire up the toggle to call update_gacha_record with toggled isWon:

```tsx
// Near other mutation handlers:
const handleToggleWon = async (id: number, newIsWon: boolean) => {
  // Find the record to get current values
  const record = records.find(r => r.id === id)
  if (!record) return
  await updateMutation.mutateAsync({
    id,
    itemName: record.itemName,
    itemType: record.itemType,
    bannerType: record.bannerType,
    starRating: record.starRating,
    recordDate: record.recordDate,
    isWon: newIsWon,
  })
  refetchStats()
  refetchRecords()
}
```

Pass it to RecordTable:
```tsx
<RecordTable
  records={records}
  total={total}
  page={page}
  pageSize={pageSize}
  onPageChange={setPage}
  onToggleWon={handleToggleWon}
  onDelete={...}
  onSave={...}
/>
```

- [ ] **Step 4: Update updateMutation type to include bannerType**

```typescript
const updateMutation = useTauriMutation<
  boolean,
  { id: number; itemName: string; itemType: string; bannerType: string; starRating: number; recordDate: string; isWon: boolean }
>('update_gacha_record')
```

- [ ] **Step 5: TypeScript check**

```bash
cd frontend && npx tsc --noEmit 2>&1
```
Expected: No errors.

- [ ] **Step 6: Commit**

```bash
git add frontend/components/RecordTable.tsx frontend/pages/Gacha.tsx
git commit -m "feat: clickable is_won toggle in RecordTable"
```

---

### Task 5: Frontend — Banner tabs + per-banner stats

**Files:**
- Modify: `frontend/pages/Gacha.tsx`

- [ ] **Step 1: Add BannerStats interface and tab state**

```typescript
interface BannerStats {
  bannerType: string
  totalPulls: number
  fiveStarCount: number
  lostCount: number
  currentPity: number
  avgPullsPerFiveStar: number
}

// Update GachaStats interface:
interface GachaStats {
  totalPulls: number
  fiveStarCount: number
  lostCount: number
  currentPity: number
  avgPullsPerFiveStar: number
  byBanner: BannerStats[]
}
```

Add tab state:
```typescript
const [bannerTab, setBannerTab] = useState<string>('全部')

// 切游戏或切 tab 时重置页码
useEffect(() => {
  setPage(1)
  setBannerTab('全部')
}, [currentGame])
```

- [ ] **Step 2: Define tab config per game**

```typescript
const BANNER_TABS: Record<string, string[]> = {
  starrail: ['全部', '角色活动', '光锥活动', '常驻'],
  genshin: ['全部', '角色活动', '武器活动', '常驻'],
}

function getGameKey(game: string): string {
  return game === 'genshin' ? 'genshin' : 'starrail'
}
```

- [ ] **Step 3: Pass banner filter to query**

```typescript
const { data: recordsResponse, refetch: refetchRecords } = useTauriQuery<GachaRecordsResponse>(
  'get_gacha_records',
  {
    gameKind: currentGame,
    page,
    pageSize,
    banner: bannerTab !== '全部' ? bannerTab : null,
  }
)
```

- [ ] **Step 4: Update stat cards to use per-banner data**

```typescript
// Find current tab's stats
const currentBannerStats = bannerTab !== '全部'
  ? stats?.byBanner?.find(b => b.bannerType.includes(bannerTab))
  : null

// Determine which stats to show
const displayStats = currentBannerStats || stats

// Build subtitle breakdown
const bannerBreakdown = stats?.byBanner
  ?.map(b => `${b.bannerType.replace('跃迁', '').replace('祈愿', '')} ${b.totalPulls}`)
  .join(' / ') ?? ''
```

Update StatCards to use displayStats and show breakdown sub when on "全部" tab:
```tsx
<StatCard
  label="累计抽数"
  value={displayStats?.totalPulls?.toLocaleString() ?? '--'}
  sub={bannerTab === '全部' && bannerBreakdown ? bannerBreakdown : undefined}
/>
```

For pity stat, change "距保底 X 抽" since hard pity differs between character (90) and weapon (80):
```tsx
<StatCard
  label="当前保底"
  value={currentBannerStats?.currentPity ?? stats?.currentPity ?? '--'}
  sub={currentBannerStats
    ? (bannerTab.includes('光锥') || bannerTab.includes('武器'))
      ? `距保底 ${80 - currentBannerStats.currentPity} 抽`
      : `距保底 ${90 - currentBannerStats.currentPity} 抽`
    : undefined
  }
  subColor="#D4433B"
/>
```

- [ ] **Step 5: Add tabs UI above stat cards**

Add before the stat cards grid:

```tsx
{/* 卡池 Tabs */}
<div className="flex gap-1 border-b border-gray-200 pb-2">
  {BANNER_TABS[getGameKey(currentGame)].map(tab => (
    <button
      key={tab}
      onClick={() => { setBannerTab(tab); setPage(1) }}
      className={`px-4 py-1.5 text-sm rounded-t-lg transition-colors ${
        bannerTab === tab
          ? 'bg-white text-gray-900 font-medium border border-b-0 border-gray-200 -mb-[1px]'
          : 'text-gray-500 hover:text-gray-700 cursor-pointer'
      }`}
    >
      {tab}
    </button>
  ))}
</div>
```

- [ ] **Step 6: TypeScript check**

```bash
cd frontend && npx tsc --noEmit 2>&1
```
Expected: No errors.

- [ ] **Step 7: Commit**

```bash
git add frontend/pages/Gacha.tsx
git commit -m "feat: add banner tabs and per-banner stats to Gacha page"
```

---

### Task 6: Frontend — Filter bar (star rating + sort + page size)

**Files:**
- Modify: `frontend/pages/Gacha.tsx`
- Modify: `frontend/components/RecordTable.tsx` (pass sort/filter props)

- [ ] **Step 1: Add filter state to Gacha.tsx**

```typescript
const [starFilter, setStarFilter] = useState<number | null>(null)
const [sortBy, setSortBy] = useState<string>('date')
const [sortOrder, setSortOrder] = useState<string>('desc')
const [pageSize, setPageSize] = useState<number>(20)
```

- [ ] **Step 2: Pass filter params to query**

```typescript
const { data: recordsResponse, refetch: refetchRecords } = useTauriQuery<GachaRecordsResponse>(
  'get_gacha_records',
  {
    gameKind: currentGame,
    page,
    pageSize,
    banner: bannerTab !== '全部' ? bannerTab : null,
    starFilter,
    sortBy,
    sortOrder,
  }
)
```

- [ ] **Step 3: Add filter bar UI between tabs and stat cards**

```tsx
{/* 筛选栏 */}
<div className="flex gap-4 items-center flex-wrap">
  <div className="flex items-center gap-2">
    <label className="text-xs text-gray-500">星级</label>
    <select
      value={starFilter ?? ''}
      onChange={e => { setStarFilter(e.target.value ? Number(e.target.value) : null); setPage(1) }}
      className="px-2 py-1 border border-gray-200 rounded-lg text-sm"
    >
      <option value="">全部</option>
      <option value="5">5★</option>
      <option value="4">4★</option>
      <option value="3">3★</option>
    </select>
  </div>

  <div className="flex items-center gap-2">
    <label className="text-xs text-gray-500">排序</label>
    <select
      value={`${sortBy}-${sortOrder}`}
      onChange={e => {
        const [by, order] = e.target.value.split('-')
        setSortBy(by)
        setSortOrder(order)
        setPage(1)
      }}
      className="px-2 py-1 border border-gray-200 rounded-lg text-sm"
    >
      <option value="date-desc">日期 ↓</option>
      <option value="date-asc">日期 ↑</option>
      <option value="star-desc">星级 ↓</option>
      <option value="star-asc">星级 ↑</option>
    </select>
  </div>

  <div className="flex items-center gap-2">
    <label className="text-xs text-gray-500">每页</label>
    <select
      value={pageSize}
      onChange={e => { setPageSize(Number(e.target.value)); setPage(1) }}
      className="px-2 py-1 border border-gray-200 rounded-lg text-sm"
    >
      <option value={20}>20</option>
      <option value={50}>50</option>
      <option value={100}>100</option>
    </select>
  </div>
</div>
```

- [ ] **Step 4: TypeScript check**

```bash
cd frontend && npx tsc --noEmit 2>&1
```
Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add frontend/pages/Gacha.tsx
git commit -m "feat: add filter bar with star rating, sort, and page size controls"
```

---

### Task 7: Frontend — Pagination with page number buttons

**Files:**
- Modify: `frontend/components/RecordTable.tsx`

- [ ] **Step 1: Add page number buttons**

Replace the basic prev/next pagination with numbered page buttons:

```tsx
{/* 分页 */}
<div className="flex items-center justify-between px-4 py-2.5 border-t border-gray-100 text-sm text-gray-500">
  <span>共 {total} 条 / {totalPages} 页</span>
  <div className="flex items-center gap-1">
    <button
      onClick={() => onPageChange(page - 1)}
      disabled={page <= 1}
      className="px-2.5 py-1 rounded border border-gray-200 disabled:opacity-30 hover:bg-gray-50 text-xs"
    >
      上一页
    </button>
    {generatePageNumbers(page, totalPages).map((n, i) =>
      n === '...' ? (
        <span key={`ellipsis-${i}`} className="px-1 text-gray-300">...</span>
      ) : (
        <button
          key={n}
          onClick={() => onPageChange(n as number)}
          className={`w-7 h-7 rounded text-xs ${
            page === n
              ? 'bg-blue-500 text-white'
              : 'border border-gray-200 hover:bg-gray-50'
          }`}
        >
          {n}
        </button>
      )
    )}
    <button
      onClick={() => onPageChange(page + 1)}
      disabled={page >= totalPages}
      className="px-2.5 py-1 rounded border border-gray-200 disabled:opacity-30 hover:bg-gray-50 text-xs"
    >
      下一页
    </button>
  </div>
</div>
```

Add helper function at the top of the component (or as a module-level function):

```typescript
function generatePageNumbers(current: number, total: number): (number | '...')[] {
  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => i + 1)
  }
  const pages: (number | '...')[] = [1]
  if (current > 3) pages.push('...')
  const start = Math.max(2, current - 1)
  const end = Math.min(total - 1, current + 1)
  for (let i = start; i <= end; i++) pages.push(i)
  if (current < total - 2) pages.push('...')
  if (total > 1) pages.push(total)
  return pages
}
```

- [ ] **Step 2: TypeScript check**

```bash
cd frontend && npx tsc --noEmit 2>&1
```
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add frontend/components/RecordTable.tsx
git commit -m "feat: page number navigation in pagination"
```

---

### Task 8: Final verification

- [ ] **Step 1: Full backend check**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1
```
Expected: All tests pass.

- [ ] **Step 2: Full frontend check**

```bash
cd frontend && npx tsc --noEmit 2>&1
```
Expected: No errors.

- [ ] **Step 3: Verify end-to-end**

Run `pnpm tauri dev` and check:
1. Tabs switch between 全部/角色活动/光锥活动/常驻
2. Stats update when switching tabs
3. Star filter works
4. Sorting works (date ASC/DESC, star ASC/DESC)
5. Page size selector works (20/50/100)
6. Pagination with page numbers works
7. Click "欧 ✓" / "歪了" toggles isWon immediately
8. Card pool column shows colored tags

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: complete gacha banner redesign with tabs, filtering, pagination"
```

---

### Spec Coverage Check

| Spec Requirement | Task |
|---|---|
| Banner tabs (全部/角色活动/光锥活动/常驻) | Task 5 |
| Per-banner stats in get_gacha_stats | Task 2 |
| Stat cards update with tab | Task 5 Step 4 |
| Banner_type column in table | Task 3 |
| Colored banner tags | Task 3 Step 2 |
| is_won clickable toggle | Task 4 |
| Star rating filter | Task 6 |
| Sort (date/star, asc/desc) | Task 6 |
| Page size selector (20/50/100) | Task 6 |
| Pagination page numbers | Task 7 |
| DB migration (done) | Pre-existing |
| Import saves banner_type (done) | Pre-existing |
| Overview stays combined | Not changed |
