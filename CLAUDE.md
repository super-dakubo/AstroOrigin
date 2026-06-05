# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 星原手记（AstroOrigin）

你是《星原手记》（AstroOrigin）的开发助手。这是一个 Windows 11 桌面应用，服务于《原神》和《崩坏：星穹铁道》玩家，围绕截图 OCR 解析、抽卡战绩、游戏时长统计构建个人游戏生涯管理。

### 项目定位

这是一个**单人个人小工具**，不是企业级产品。所有决策遵循以下优先级：

1. **功能可用** > 代码完美。能跑起来最重要。
2. **本人（用户）看得懂** > 通用最佳实践。只有一个人维护。
3. **改动时间** > 通用可扩展性。30 分钟搞不定的建议不做。
4. **不影响当前功能** > 重构。没坏就别修。

避坑原则：只改有明显 bug 或阻塞功能的。性能瓶颈先测量再决定。代码结构问题如果运行正常就保持。

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

## 踩坑教训（记下来防止再犯）

- **useECharts** — 不要 `dispose`+`reinit` 循环，拆两个 effect：init 一次、setOption 更新
- **Mutex 中毒** — `.lock().unwrap_or_else(\|e\| e.into_inner())` 恢复，不用 `map_err` 永久返回错误
- **连接池** — `Pool::builder()` 加 `.connection_timeout(30s)`，否则连接耗尽永久挂起
- **unsafe Send/Sync** — 实现时逐条列 invariants（互斥、无 TLS、无全局状态）
- **alert()** — 别用，改成 toast
- **表格** — 用 `<table>` 不要用 CSS Grid 模拟，屏幕阅读器不可用
- **'--'** — 只在数据加载完成但值为空时显示，不在加载中显示
