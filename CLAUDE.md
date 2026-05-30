# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 星原手记（AstroOrigin）

你是《星原手记》（AstroOrigin）的开发助手。这是一个 Windows 11 桌面应用，服务于《原神》和《崩坏：星穹铁道》玩家，围绕截图 OCR 解析、抽卡战绩、游戏时长统计构建个人游戏生涯管理。

---

## 技术栈

- **桌面框架**：Tauri 2.x（Rust 后端 + WebView2 前端）
- **前端**：React 18+、TypeScript、Vite
- **组件库**：HeroUI（`@heroui/react`），基于 Tailwind CSS
- **图标**：Lucide（`lucide-react`）
- **路由**：React Router v6（**必须用 HashRouter，不能用 BrowserRouter**，Tauri 使用 `file://` 协议）
- **状态管理**：Zustand
- **数据请求**：`@tanstack/react-query`，通过 `useTauriQuery`/`useTauriMutate` hooks 调用 invoke
- **图表**：`echarts`（通过 `useRef` + 原生 `echarts.init` 初始化，**不用 `echarts-for-react`**）
- **样式**：Tailwind CSS + HeroUI 内置样式
- **包管理**：pnpm
- **数据库**：SQLite（`rusqlite` + `r2d2` 连接池）

## Rust 后端约束

- 所有传给前端的结构体必须标注 `#[derive(Serialize, Deserialize)] #[serde(rename_all = "camelCase")]`
- 数据库操作必须封装在 `tokio::task::spawn_blocking` 中调用（`rusqlite::Connection` 非 Send）
- 使用 `r2d2` 连接池，不直接用 `Mutex<Connection>`
- `pure-onnx-ocr` 引擎内部使用了 `RefCell`（非 `Send + Sync`），通过 `Mutex` + `unsafe impl Send/Sync` 包装为线程安全
- 使用 `anyhow` 处理错误，Tauri 命令返回 `Result<T, String>`
- 图片处理仅用 `image` crate（加载、裁剪），不用 `imageproc`（编译过重）
- 进程列表扫描用 `sysinfo`，日期时间用 `chrono`

## 前端代码规范

- 所有组件使用 TypeScript，显式定义 Props 类型
- HeroUI 组件优先使用，避免原生 HTML 堆砌
- 图表用 `useRef` + `echarts.init`，在 `useEffect` 中绑定，return 中 dispose
- 数据请求通过 `useTauriQuery<T>(command, args)` hook，不直接调用 `invoke()`
- Zustand store 按功能域拆分（`gameStore`、`gachaStore` 等），不使用 prop drilling
- 仅浅色模式，不实现深色模式切换
- 所有路由使用 HashRouter：

```tsx
import { HashRouter, Routes, Route } from 'react-router-dom';
```

## 项目结构

```text
d:\code\AstroOrigin/
├── frontend/                    # React 前端 (Vite root)
│   ├── index.html               # 入口 HTML（Vite root 指向 frontend/）
│   ├── App.tsx                  # HashRouter + HeroUIProvider + QueryClientProvider
│   ├── main.tsx                 # ReactDOM 挂载
│   ├── App.css                  # Tailwind 指令
│   ├── pages/                   # 路由页面组件
│   ├── components/              # 可复用 UI 组件
│   ├── hooks/                   # 自定义 hooks
│   ├── stores/                  # Zustand stores
│   └── lib/                     # 常量、类型定义
├── src-tauri/                   # Rust 后端 (Tauri)
│   ├── assets/models/           # PP-OCRv4 ONNX 模型文件（~15MB）
│   ├── src/
│   │   ├── lib.rs               # Tauri Builder + command 注册
│   │   ├── main.rs              # 程序入口
│   │   ├── commands/            # Tauri commands（按模块分）
│   │   ├── game/                # GameKind 枚举、特征定义
│   │   ├── ocr.rs               # 通用 OCR 入口（ocr_image → Vec<OcrWord>）
│   │   ├── paddle.rs            # PP-OCRv4 引擎封装（SafeOcrEngine + Mutex）
│   │   ├── db.rs                # r2d2 连接池 + 表迁移
│   │   └── error.rs             # anyhow 包装
│   ├── tauri.conf.json          # 安全配置（CSP 锁定，无危险权限）
│   └── Cargo.toml               # Rust 依赖（主版本锁死）
├── postcss.config.cjs           # PostCSS（注意是 .cjs，因为 package.json 有 type: module）
└── docs/                        # 设计规格、技术栈、实施计划
```

## 常用命令

```bash
# 开发
pnpm tauri dev          # 启动 Tauri 开发模式（Vite HMR + Rust 热重载）

# 构建
cargo check --manifest-path src-tauri/Cargo.toml    # Rust 编译检查
npx tsc --noEmit                                     # TypeScript 检查
pnpm build                                           # 前端构建
pnpm tauri build                                     # 打包成 .msi / .exe（产物在 src-tauri/target/release/bundle/）

# 依赖
pnpm add <pkg>                                       # 加前端依赖
pnpm approve-builds <pkg>                            # 批准构建脚本（HeroUI 需要）

# 图标
pnpm tauri icon <svg-path>                           # 生成应用图标
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

- **文件对话框 + 截图导入**：用户通过文件选择器导入截图，不走拖拽
- **OCR 引擎**：PP-OCRv4（ONNX + tract 纯 Rust 推理），**已替换 Windows.Media.Ocr**。模型文件（~15MB）在 `assets/models/`，引擎通过 `OnceLock` + `Mutex` 延迟初始化，线程安全
- **OCR 管线**：整图 OCR（`ocr::ocr_image`） → 坐标聚类行列 → 按表头 X 范围分列 → 模糊匹配 → 去重入库（全部在 `spawn_blocking` 中执行）
- **表格解析**：从表头行计算列边界区间，所有数据行共用此区间做列分配。PP-OCRv4 输出按词组而非单字，列检测兼容 1 词/列和 N 字/列两种模式
- **进度上报**：每张图 4 阶段（检测→识别→解析→入库），通过全局 `APP_HANDLE` + Tauri event 上报，前端用百分比圆环 + 展开式进度条展示
- **批量导入**：`import_gacha_screenshots` 命令支持多文件选择，顺序处理，失败跳过
- **宽容策略**：OCR 识别不准时保留空字段入库，用户可通过 ✏️ 编辑修复。不因单格识别失败丢弃整行
- **去重**：数据库 UNIQUE 索引 `(game_kind, item_name, record_date)`，`INSERT OR IGNORE`
- **游戏切换**：使用 Zustand store + CSS 变量，不重启应用
- **安全**：`tauri.conf.json` 锁死 CSP，不开启 `shell.open`，使用 capabilities 替代 allowlist

## AI 自检清单（每次生成代码后逐条检查）

1. 路由是否用了 HashRouter？（Tauri 不允许 BrowserRouter）
2. 图表是否用了 echarts-for-react？→ 改为 `useRef` + 原生初始化
3. Rust 结构体是否加了 `#[serde(rename_all = "camelCase")]`？
4. 数据库操作是否在 `spawn_blocking` 中执行？
5. 数据库连接用 r2d2 连接池，不用裸 `Mutex<Connection>`？
6. `pure-onnx-ocr` 的 `OcrEngine` 是否用了 `Mutex` + `unsafe impl Send/Sync` 包装？
7. tauri.conf.json 是否未开启 shell.open 或其他危险权限？
8. Rust 错误是否用 anyhow？返回前转成 `String`
9. 是否擅自升降了 package.json / Cargo.toml 主版本号？
10. `postcss.config` 是否需要 `.cjs` 后缀？（项目 `type: module`）
