# 全栈健康检查执行计划

> **For agentic workers:** 使用 Workflow 并行执行。步骤使用复选框 (`- [ ]`) 追踪。

**目标：** 对星原手记进行全面健康检查，覆盖 UI/交互、动画、前端代码、Rust 后端+架构四个维度

**架构：** 并行专家团模式（方案 A）—— 4 个子智能体同时独立审查，最后合成报告

**参考 Spec：** `docs/superpowers/specs/2026-06-05-fullstack-review-design.md`

---

### Task 1：UI/交互审查 Agent

**审查范围：** 所有 `.tsx` 页面及组件

- [ ] **Agent 输出格式**
  每个发现：`文件:行号 | 问题 | 当前表现 | 建议`

- [ ] **检查清单：**
  - 加载态/空态/错误态覆盖
  - 交互反馈（点击、悬停、切换）
  - 布局一致性（间距、对齐、网格）
  - 表单控件状态（disabled、focus、validation）
  - 键盘可访问性（tab order、focus visible）
  - 游戏切换的 UI 适配

- [ ] **输出结果** → 供合成阶段使用

### Task 2：动画审查 Agent

**审查范围：** App.css、组件内外动画、过渡

- [ ] **Agent 输出格式**
  每个发现：`类型 | 位置 | 问题 | 修复方向`

- [ ] **检查清单：**
  - 页面切换/路由过渡
  - 新增/删除行过渡
  - 动画性能风险（强制重排、GPU 合成）
  - 动画时长/缓动函数一致性
  - FLIP 缺失导致的跳变

- [ ] **输出结果** → 供合成阶段使用

### Task 3：前端代码质量 Agent

**审查范围：** 所有 `.tsx` + `.ts`

- [ ] **Agent 输出格式**
  每个发现：`文件:行号 | 严重度 (high/medium/low) | 问题 | 建议`

- [ ] **检查清单：**
  - Hooks 规则（deps arrays、条件调用）
  - Zustand store 设计
  - useTauriQuery 使用正确性
  - TypeScript 类型安全
  - 组件职责边界
  - 不必要的重渲染
  - 命名语义化

- [ ] **输出结果** → 供合成阶段使用

### Task 4：Rust 后端 + 架构 Agent

**审查范围：** 所有 `.rs`

- [ ] **Agent 输出格式**
  每个发现：`文件:行号 | 严重度 | 风险类型 | 问题 | 修复建议`

- [ ] **检查清单：**
  - 错误处理模式（anyhow、unwrap、panic 风险）
  - spawn_blocking 用法
  - r2d2 连接池模式
  - SQL 注入防护
  - 模块职责划分
  - OCR 管线容错
  - IPC 序列化成本
  - 死锁/并发风险
  - serde 标注完整性

- [ ] **输出结果** → 供合成阶段使用

### Task 5：合成报告

- [ ] **收集** 4 个 Agent 的所有发现
- [ ] **去重归类：** Critical / Improvement / Nitpick / Clean
- [ ] **写入报告：** `docs/reviews/2026-06-05-fullstack-review.md`
- [ ] **呈现给用户**
