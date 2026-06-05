# 全栈健康检查设计

> 日期：2026-06-05
> 项目：星原手记 (AstroOrigin)
> 范围：React/TypeScript 前端 + Rust/Tauri 后端

## 概述

对星原手记进行首次全面健康检查，覆盖 UI/交互、动画、前端代码、Rust 后端+架构四个维度。采用并行专家团模式（方案 A），4 个子智能体同时审计，最后合成一份结构化报告。

代码库规模：前端 ~1277 行（4 页面 + 5 组件 + stores/hooks/lib），后端 ~2100 行（gacha.rs 核心逻辑 875 行 + OCR 管线 + 数据库层）。

## 智能体设计

### Agent 1：UI/交互审查

| 属性 | 值 |
|------|-----|
| 审查文件 | Gacha.tsx, Overview.tsx, Playtime.tsx, Screenshots.tsx, Layout.tsx, RecordTable.tsx, StatCard.tsx |
| 页面组件 | 4 个（Overview/Gacha/Playtime/Screenshots） |
| 可复用组件 | 5 个（Layout/RecordTable/StatCard/GameSwitch/LuckChart） |

**检查清单：**
- [ ] 加载态 / 空态 / 错误态是否覆盖
- [ ] 交互反馈（点击、悬停、切换）是否一致
- [ ] 布局一致性（间距、对齐、网格）
- [ ] 表单控件状态（disabled、focus、validation）
- [ ] 键盘可访问性（tab order、focus visible）
- [ ] 游戏切换（Genshin/StarRail）的 UI 适配

**输出格式：** `文件:行号 | 问题 | 当前表现 | 建议`

### Agent 2：动画审查

| 属性 | 值 |
|------|-----|
| 审查文件 | App.css, 所有组件 TSX, LuckChart.tsx |
| 关注点 | CSS 过渡、路由动画、echart 动画、行增删动画 |

**检查清单：**
- [ ] 页面切换 / 路由过渡是否流畅
- [ ] 新增/删除行是否有过渡动画
- [ ] 动画性能风险（强制重排、无 GPU 合成）
- [ ] 动画时长/缓动函数一致性
- [ ] 无过渡导致的突然跳变（FLIP 缺失）
- [ ] CSS transition 在哪些属性上触发（避免 layout-triggering props）

**输出格式：** `类型 | 位置 | 问题 | 修复方向`

### Agent 3：前端代码质量

| 属性 | 值 |
|------|-----|
| 审查文件 | 所有 `.tsx` + `.ts` |
| 检查层次 | React hooks / Zustand / TypeScript / 组件职责 / 性能 |

**检查清单：**
- [ ] Hooks 规则（deps arrays 完备、无条件调用）
- [ ] Zustand store 设计（selector 粒度、不可变更新）
- [ ] `useTauriQuery`/`useTauriMutate` 使用正确性
- [ ] TypeScript 类型安全（any 滥用、类型导出）
- [ ] 组件职责边界（单一职责？文件过大？）
- [ ] 不必要的重渲染（memo/useMemo/useCallback 使用合理性）
- [ ] 命名一致性和语义化

**输出格式：** `文件:行号 | 严重度 (high/medium/low) | 问题 | 建议`

### Agent 4：Rust 后端 + 架构

| 属性 | 值 |
|------|-----|
| 审查文件 | 所有 `.rs`（~2100 行） |
| 关注模块 | gacha.rs(875行), ocr.rs, db.rs, game/, commands/, lib.rs, main.rs, paddle.rs |

**检查清单：**
- [ ] 错误处理模式（anyhow → String、panic 风险、unwrap 滥用）
- [ ] `spawn_blocking` 用法正确性（非 Send 类型、闭包捕获避免）
- [ ] `r2d2` 连接池使用模式（生命周期、获取释放）
- [ ] SQL 注入防护（参数化查询 vs 字符串拼接）
- [ ] 模块职责划分（commands/ vs game/ vs db.rs）
- [ ] OCR 管线错误处理（容错降级策略）
- [ ] IPC 序列化成本控制（大数据量接口）
- [ ] 死锁/并发风险
- [ ] `#[serde(rename_all = "camelCase")]` 标注完整性

**输出格式：** `文件:行号 | 严重度 | 风险类型 | 问题 | 修复建议`

## 执行方式

4 个 Agent 通过 Workflow 并行启动，每个 Agent 使用 `isolation: "worktree"` 在独立工作区中读取完整代码库。互不依赖，同时执行。

预计单个 Agent 输出 5-15 条发现，4 个 Agent 合计 20-60 条。

## 合成报告结构

4 个 Agent 完成后，汇总去重归类：

### 1. Critical（必须修）
- Bug 或会导致运行时错误的代码
- 安全漏洞（SQL 注入风险等）
- 数据丢失风险

### 2. Improvement（值得改）
- 可维护性改进（重构、解耦）
- 性能优化点
- 错误处理完善

### 3. Nitpick（可选优化）
- 命名/风格不一致
- 轻微代码异味
- 文档或注释缺失

### 4. Clean（保持）
- 值得肯定的设计模式
- 良好的架构决策

最终报告写入 `docs/reviews/2026-06-05-fullstack-review.md`。

## 输出格式

合成报告每条发现格式：

```markdown
- **类型**: Critical | Improvement | Nitpick | Clean
- **位置**: `文件:行号`
- **来源**: Agent 名称
- **问题**: 描述
- **建议**: 修复/改进方向
```

## 不做的事情

- 不改代码（审查性质，仅输出报告）
- 不重构（指出问题，不直接改动）
- 不重复 CLAUDE.md 中已有的项目约定检查（如 HashRouter、echarts 用法、serde annotate）—— 这些由自检清单覆盖
