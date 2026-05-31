# 抽卡记录卡池分页设计

## 背景

当前抽卡记录将所有卡池记录混在一起显示，统计也是合计的。但原神和星铁的卡池有明确分类（角色活动/武器活动/常驻），不同卡池的保底独立计算，歪/不歪的概念也因卡池而异。用户需要在抽卡记录页面按卡池分类查看，以及按卡池分别统计。

## 范围

- **抽卡记录（Gacha）页面**：重新设计，加入卡池 tabs、筛选排序、分页
- **总览（Overview）页面**：保持简易，仅显示合计统计
- **后端**：新增 banner_type 字段、分页查询、筛选排序支持
- **导入**：保存 OCR 提取的卡池信息

不包含：
- LuckChart 按卡池分色（后续迭代）
- 原神武器池定轨逻辑（手动标记歪/不歪即可）

## 数据模型

### gacha_records 表新增字段

```sql
ALTER TABLE gacha_records ADD COLUMN banner_type TEXT NOT NULL DEFAULT '';
```

存储 OCR 提取的原始卡池名（如"角色活动跃迁"、"常驻祈愿"等）。

### GachaRecord 结构体

```rust
pub struct GachaRecord {
    pub id: i64,
    pub game_kind: String,
    pub item_name: String,
    pub item_type: String,    // "角色" / "光锥" / "武器"
    pub star_rating: i32,
    pub record_date: String,
    pub is_won: bool,
    pub banner_type: String,  // 新增
}
```

### GachaStats 拓展

```rust
pub struct BannerStats {
    pub banner_type: String,
    pub total_pulls: i64,
    pub five_star_count: i64,
    pub lost_count: i64,
    pub current_pity: i32,
    pub avg_pulls_per_five_star: f64,
}

pub struct GachaStats {
    pub total_pulls: i64,
    pub five_star_count: i64,
    pub lost_count: i64,
    pub current_pity: i32,
    pub avg_pulls_per_five_star: f64,
    pub by_banner: Vec<BannerStats>,  // 新增：按卡池拆分的统计
}
```

## 后端 API 变更

### get_gacha_records

当前：
- 参数：`game_kind, page, page_size`
- 返回：`{ records: GachaRecord[], total: i64 }`
- 排序：固定 `record_date DESC, id DESC`

改为：
- 新增参数：`banner: Option<String>` — 按卡池筛选
- 新增参数：`star_filter: Option<i32>` — 按星级筛选
- 新增参数：`sort_by: Option<String>` — "date" / "star"
- 新增参数：`sort_order: Option<String>` — "asc" / "desc"
- 返回不变

### get_gacha_stats

当前：返回合计统计。

改为：新增按 banner_type GROUP BY 的统计，返回 `by_banner: Vec<BannerStats>`。

前端 tab 切换时，直接用 `by_banner` 中对应项的统计，不需要重新请求。

### 导入

导入时保存 `c[2]`（跃迁类型）到 `banner_type` 字段。已在 #1 实现。

### 更新命令

update_gacha_record 支持编辑 banner_type。

## 前端 UX 设计

### 页面布局

```
┌─────────────────────────────────────────────┐
│  抽卡记录                    [+导入][+批量] │
│  帕姆帮你记着每一跃                          │
├─────────────────────────────────────────────┤
│  [全部] [角色活动] [光锥活动] [常驻]         │ ← Tabs
├─────────────────────────────────────────────┤
│  累计抽数  │ 5⭐出货 │ 当前保底 │ 歪率       │ ← 统计卡片
│  328       │ 5      │ 32      │ 40%        │   联动 tab
│  角色 210   │ 均65抽 │ 角色12   │ 2/5       │
├─────────────────────────────────────────────┤
│  星级:[全部▼] 排序:[日期▼] 每页:[20▼]      │ ← 筛选栏
├─────────────────────────────────────────────┤
│  日期  │种类│ 卡池   │ 物品   │⭐│结果│  │ ← 表格
│  05-01 │角色│角色活动│ 貊泽   │4 │欧✓│✕ │   可点击
│  04-28 │角色│角色活动│布洛妮娅│5 │歪了│✕ │   切换歪/不歪
│  04-20 │光锥│光锥活动│ 齐颂   │3 │ - │✕ │
├─────────────────────────────────────────────┤
│  共128条 / 7页    [上一页] 1 2 3 [下一页]   │ ← 分页
└─────────────────────────────────────────────┘
```

### Tabs

- 共 4 个 tab：全部、角色活动、光锥活动（原神：武器活动）、常驻
- 切换时发送新查询（`banner` 参数），联动统计、表格、图表
- 当前 tab 高亮

### 统计卡片

4 张卡片，与当前布局一致。数据跟随 tab 切换：
- 累计抽数：副标题展开各卡池占比（如"角色池 210 / 光锥池 80 / 常驻 38"）
- 5⭐出货：平均值
- 当前保底：副标题展开各卡池保底值
- 歪率：副标题显示 n/m

### 筛选栏

表格上方一行，包含：
- 星级筛选 dropdown：全部 / 5★ / 4★ / 3★
- 排序 dropdown：日期↓ / 日期↑ / 星级↓ / 星级↑
- 每页条数 dropdown：20 / 50 / 100（默认 20）

### 表格

列定义：

| 列 | 宽度 | 说明 |
|----|------|------|
| 日期 | 1.5fr | 显示格式 YYYY-MM-DD HH:MM |
| 种类 | 0.6fr | 角色/光锥/武器 |
| 卡池 | 0.8fr | 带颜色标签，角色活动蓝、光锥活动紫、常驻灰 |
| 物品 | 2.5fr | 可编辑（点击铅笔） |
| 星级 | 50px | ★ 重复 |
| 结果 | 60px | 5★显示"欧✓"/"歪了"（可点击切换），其他显示"-" |
| 删 | 30px | 删除按钮 |

### 歪/不歪交互

- 表格中 5★ 行的"欧 ✓"或"歪了"直接点击切换
- 点击即调用 `update_gacha_record` 更新 `is_won`
- 不需要进入编辑模式
- 切换时背景色变化反馈（绿色/红色）

### 编辑模式

点击物品名的铅笔图标进入编辑模式，可以编辑：
- 物品名、种类、卡池、星级、日期、歪/不歪

### 分页

- 默认每页 20 条
- 用户可通过筛选栏 dropdown 改为 50/100
- 显示总条数和页码导航
- 页码按钮：上一页、数字、下一页

### 总览页

保持现有 4 张统计卡片布局，显示合计统计（不分卡池）。仅在抽卡页提供细化的按卡池查看体验。

## 游戏差异

|  | 星穹铁道 | 原神 |
|--|---------|------|
| Tab 标签 | 角色活动 / 光锥活动 / 常驻 | 角色活动 / 武器活动 / 常驻 |
| 卡池列标签 | 角色活动/光锥活动/常驻 | 角色活动/武器活动/常驻 |
| 种类列 | 角色 / 光锥 | 角色 / 武器 |
| OCR 卡池名 | 角色活动跃迁 / 光锥活动跃迁 / 常驻跃迁 | 角色活动祈愿 / 武器活动祈愿 / 常驻祈愿 |

Tab 标签和卡池标签固定为中文简称，不随游戏变。背后的 banner_type 存 OCR 原始文本。

## 实现顺序

1. DB migration: banner_type 字段（已完成）
2. Rust 后端：get_gacha_records 筛选/排序参数 + get_gacha_stats 按卡池分组
3. Rust 后端：导入保存 banner_type（已完成）
4. 前端：GachaRecord interface + RecordTable 加卡池列 + 歪/不歪点击切换
5. 前端：Tabs + 统计卡片联动
6. 前端：筛选栏（星级/排序/分页大小）
7. 前端：分页页码导航
8. 重新导入数据（或手动修正已有记录的 banner_type）

## 未包含（以后再说）

- LuckChart 按卡池分色
- 原神武器池定轨追踪
- 自动检测歪/不歪（基于 banner_type + 物品名）
- 导出/分享功能
