# Gacha 页面面板/列表布局重设计

> 日期：2026-06-05
> 状态：设计已确认

## 问题

1. **欧非曲线占用过多垂直空间**，导致表格被推到下方，用户需要滚动才能操作分页
2. **切分页后页面跳回顶部**，打断阅读流
3. **分页器当前只显示 3 个页码**（当前页 ± 1），大页数时导航不便

## 设计方案

### 页面结构（自上而下）

```
┌─ 抽卡记录 ────────────────── [+导入] [+批量] ─┐
├─ Banner Tabs: [全部] [角色] [光锥] [常驻] ─────┤
├─ 统计卡片 (4列网格，常驻) ─────────────────────┤
├─ Segmented Control: [📊 面板] [📋 列表] ──────┤
├─ 内容区 ──────────────────────────────────────┤
│  面板 tab: 欧非曲线 LuckChart                 │
│  列表 tab: 筛选栏 + RecordTable (含分页)       │
└──────────────────────────────────────────────-┘
```

### Segmented Control

- 类似 `GameSwitch` 的按钮组样式
- 两个选项：`面板` 和 `列表`，带图标
- 居中显示在统计卡片下方
- 选中项：白色背景 + 浅阴影；未选中：灰色文字
- 切换时内容区无过渡动画（instant swap，无闪烁）

### 面板 Tab

- 只显示 `LuckChart` 组件（欧非曲线）
- 高度保持 192px (`h-48`)，曲线始终可见
- 不显示筛选栏和表格
- 不影响 Banner Tabs 和统计卡片

### 列表 Tab

- 显示筛选栏（星级/排序/每页）+
- 显示 `RecordTable`（含分页）
- 筛选栏和表格之间无图表挤压，列表 tab 首次加载时表格就在屏幕可见范围内
- 切分页时滚动位置稳定在表格附近，不会跳回页面顶部

### 分页器优化

- 改为显示当前页 ± 2（共 5 个页码数字）
- 始终显示第一页和最后一页
- 中间用省略号（`...`）分隔
- 修改 `generatePageNumbers` 函数（`frontend/components/RecordTable.tsx`）

## 改动文件

| 文件 | 改动 |
|------|------|
| `frontend/pages/Gacha.tsx` | 核心改动：提取组件、加入 segmented control、条件渲染 |
| `frontend/components/RecordTable.tsx` | `generatePageNumbers` 修改为 ±2 |
| `frontend/lib/constants.ts` | 可能需要新常量（如视角状态枚举） |

## 不做的事情

- 不改 `LuckChart` 组件 — 视觉和逻辑保持不变
- 不改 `RecordTable` 组件的 props — 复用现有接口
- 不改后端 — 纯前端改动
- 不新增依赖 — 只使用现有 tailwind 类

## 技术实现要点

### 状态管理

Gacha 页面新增 `viewMode` 状态：

```typescript
type ViewMode = 'panel' | 'list'
const [viewMode, setViewMode] = useState<ViewMode>('panel')
```

### 条件渲染

```tsx
{viewMode === 'panel' ? (
  <LuckChart records={chartData} />
) : (
  <>
    <FilterBar ... />
    <RecordTable ... />
  </>
)}
```

### 筛选栏提取

为保持组件职责清晰，筛选栏的 JSX 建议提取为局部变量或子组件（内联，不新增文件）。

### Segmented Control 样式

参考 GameSwitch 的按钮组样式：

```tsx
<div className="inline-flex bg-gray-100 rounded-lg p-0.5">
  <button className={`px-6 py-1.5 text-sm rounded-md ...`}>📊 面板</button>
  <button className={`px-6 py-1.5 text-sm rounded-md ...`}>📋 列表</button>
</div>
```

### 切页跳动修复

列表 tab 中表格就在筛选栏下方，无需额外滚动修复。如有个别场景仍有跳动，可在 `RecordTable` 外层加 `scroll-mt-4` 作为保底。
