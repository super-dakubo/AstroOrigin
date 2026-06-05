# AstroOrigin — 星原手记

《原神》和《崩坏：星穹铁道》的多功能桌面伴侣应用，基于 Tauri 2.x。

## 功能

- **抽卡记录管理** — 截图 OCR 自动解析，包含欧非曲线、统计面板、分页列表
- **游戏时长统计** — 自动追踪游戏运行时间
- **截图管理** — 游戏截图打标签、OCR 全文检索
- **双游戏支持** — 原神 / 星铁一键切换

## 技术栈

- **桌面框架**: Tauri 2.x (Rust + WebView2)
- **前端**: React 18, TypeScript, Vite, Tailwind CSS, HeroUI
- **状态管理**: Zustand + React Query
- **OCR**: PP-OCRv4 (ONNX 纯 Rust 推理)
- **数据库**: SQLite (rusqlite + r2d2)

## 开始使用

### 前置条件

- Rust 工具链（rustup 安装）
- Node.js 18+
- pnpm (`npm install -g pnpm`)
- Windows 10/11 (WebView2)

### 开发

```bash
pnpm install
pnpm tauri dev
```

### 构建

```bash
pnpm tauri build
```

构建产物在 `src-tauri/target/release/bundle/`。

### 代码检查

```bash
npx tsc --noEmit        # TypeScript
cd src-tauri && cargo check  # Rust
pnpm format             # Prettier
```

## 项目结构

```text
frontend/          # React 前端
├── pages/         # 路由页面
├── components/    # 可复用组件
├── stores/        # Zustand stores
├── hooks/         # 通用 hooks
└── lib/           # 常量、类型
src-tauri/         # Rust 后端
├── commands/      # Tauri commands
├── game/          # 游戏枚举/特征
├── ocr/           # OCR 管线
└── db.rs          # 连接池和迁移
```

## 许可

MIT
