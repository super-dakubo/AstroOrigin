# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 星原手记（AstroOrigin）

你是《星原手记》（AstroOrigin）的开发助手。这是一个 Windows 11 桌面应用，服务于《原神》和《崩坏：星穹铁道》玩家，围绕截图 OCR 解析、抽卡战绩、游戏时长统计构建个人游戏生涯管理。

---

## 技术栈

- **桌面框架**：Tauri 2.x（Rust 后端 + WebView2 前端）
- **前端**：React 18+、TypeScript、Vite、HeroUI（`@heroui/react`）、Tailwind CSS
- **状态/路由**：Zustand + React Router v6（**必须用 HashRouter**）
- **图表**：`echarts`（`useRef` + 原生 `echarts.init`，不用 `echarts-for-react`）
- **数据请求**：`@tanstack/react-query`，通过 `useTauriQuery`/`useTauriMutate` 调用 invoke
- **包管理**：pnpm
- **数据库**：SQLite（`rusqlite` + `r2d2` 连接池）
- **格式化**：Prettier 全局安装，配置在 `.prettierrc`

## Rust 后端约束

- 结构体标注 `#[derive(Serialize, Deserialize)] #[serde(rename_all = "camelCase")]`
- 数据库操作在 `spawn_blocking` 中调用（`rusqlite::Connection` 非 Send）
- 使用 `r2d2` 连接池，不用 `Mutex<Connection>`
- 错误用 `anyhow`，Tauri 命令返回 `Result<T, String>`
- 图片处理仅用 `image` crate（加载、裁剪），不用 `imageproc`

## 常用命令

```bash
pnpm tauri dev            # 开发模式（Vite HMR + Rust 热重载）
cargo check -p src-tauri  # Rust 编译检查
npx tsc --noEmit          # TypeScript 检查
pnpm build                # 前端构建
pnpm tauri build          # 打包成 .msi / .exe
pnpm format               # Prettier 格式化前端代码
pnpm add <pkg>            # 加前端依赖
```

## 关键路径

```
frontend/
├── pages/         # 路由页面（Overview / Gacha / Playtime / Screenshots）
├── stores/        # Zustand stores（gameStore / gachaStore）
├── hooks/         # 通用 hooks（useTauriQuery / useECharts）
├── components/    # 可复用组件（Layout / StatCard / RecordTable / LuckChart / GameSwitch）
└── lib/           # 常量、类型（THEMES / ROUTES / GameKind）
src-tauri/src/
├── commands/      # Tauri commands（按模块分）
├── game/          # GameKind 枚举、特征
├── ocr.rs         # OCR 入口（ocr_image → Vec<OcrWord>）
├── paddle.rs      # PP-OCRv4 引擎封装
└── db.rs          # r2d2 连接池 + 表迁移
```

## 数据流

```
React Component → useTauriQuery (react-query) → invoke → Tauri Command
                                                          ↓
                                                     spawn_blocking
                                                          ↓
                                                    r2d2 pool → SQLite
```

## 关键设计决策

- **OCR 引擎**：PP-OCRv4（ONNX + tract 纯 Rust 推理），模型 ~15MB 在 `assets/models/`
- **OCR 管线**：整图 OCR → 坐标聚类行列 → 按表头 X 范围分列 → 模糊匹配 → 去重入库，全部在 `spawn_blocking` 中执行
- **宽容策略**：OCR 识别不准时保留空字段入库，用户可编辑修复。不因单格识别失败丢弃整行
- **去重**：UNIQUE 索引 `(game_kind, item_name, record_date)`，`INSERT OR IGNORE`
- **游戏切换**：Zustand store + CSS 变量，不重启应用
- **安全**：`tauri.conf.json` 锁死 CSP，不开启 `shell.open`

## AI 自检清单

1. 路由是否用了 HashRouter？
2. 图表是否用了 `echarts-for-react`？→ 改为 `useRef` + 原生初始化
3. Rust 结构体是否加了 `#[serde(rename_all = "camelCase")]`？
4. 数据库操作是否在 `spawn_blocking` 中执行？连接用 r2d2？
5. tauri.conf.json 是否未开启 shell.open 或其他危险权限？
6. 是否擅自升降了 package.json / Cargo.toml 主版本号？
7. `postcss.config` 是否需要 `.cjs` 后缀？（项目 `type: module`）

## 可持续优化规范

> 以下规范来自全栈健康检查的经验教训，配合记忆系统中的经验教训一起使用。

### 前端

- **图表生命周期**：`useECharts` 必须拆为两个 effect — `[]` 初始化（`init` + `resize`），`[option]` 只调 `setOption`，绝不 `dispose`+`reinit` 循环
- **Loading / Error / Empty 三态**：每个 `useTauriQuery` 必须消费 `isLoading`、`isError`、`data`。加载中显示骨架屏，错误显示提示，空数据显示占位文案。`'--'` 仅用于数据加载完成后的空值
- **语义化表格**：数据表格必须用 `<table>` + `<thead>`/`<tbody>`/`<tr>`/`<th>`/`<td>`，不用 CSS Grid 模拟表格（WCAG 1.3.1）
- **WCAG 对比度**：彩色背景上的文字必须通过 WCAG AA（普通文本 4.5:1，大文本 3:1）。金色/浅色背景上用深色字
- **表单可访问性**：每个 `<select>`/`<input>` 必须有对应的 `<label htmlFor="...">`，id 唯一
- **无 alert()**：用户提示用内联 toast（参考 Gacha 页面的错误提示模式），不用 `window.alert()`
- **共享类型**：跨组件复用的接口定义在 `lib/types.ts`，不重复声明
- **useMemo 防重算**：组件内派生数据（如 `chartData`）必须用 `useMemo`，避免每次渲染重算
- **死代码清理**：新增 Zustand store、hook、组件后，如果未被 import 则不应合入主分支

### Rust 后端

- **Mutex 中毒恢复**：`.lock()` 后用 `unwrap_or_else(|e| e.into_inner())` 恢复而非 `map_err` 返回永久错误
- **连接池超时**：`Pool::builder()` 必须设 `.connection_timeout(Duration::from_secs(30))`，避免连接耗尽时永久阻塞
- **unsafe Send/Sync 文档化**：实现 `unsafe impl Send/Sync` 时必须逐条列出确切 invariants（互斥保证、无 TLS、无全局可变状态），不能笼统说 "only accessed through Mutex"
- **无 .expect/.unwrap**：初始化路径和 setup 中用 `.context()?` 传播错误，不允许 `.expect()`/`.unwrap()`（Tauri setup 支持 `Result<(), Box<dyn Error>>`）
- **参数化查询**：SQL 用命名参数（`:name`）而非位置参数（`?1`, `?2`），防止增删条件时索引错位
- **前端字符串不泄露到后端**：后端不比较前端显示字符串（如 `"全部"`），由前端通过 `null`/`None` 控制行为
