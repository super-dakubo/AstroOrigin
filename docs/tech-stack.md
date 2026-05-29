# 技术栈清单

> 最终确认版本。初次 setup 时需验证 pnpm + HeroUI 兼容性。
> 所有主版本号已锁定，AI 不应擅自升级。

---

## 包管理与构建

| 工具 | 用途 | 版本 |
|------|------|------|
| pnpm | 包管理器 | latest |
| Vite | 前端构建工具 | ^6.0 |
| TypeScript | 类型安全 | ^5.6 |

## 前端（React）

| 依赖 | 用途 | 版本锁定 |
|------|------|----------|
| react + react-dom | UI 框架 | ^18.3.0 |
| react-router-dom | **HashRouter**（不用 BrowserRouter） | ^6.28.0 |
| zustand | 状态管理（按域拆分 store） | ^5.0.0 |
| @tanstack/react-query | 请求管理（useTauriQuery 底座） | ^5.60.0 |
| @heroui/react | 组件库（基于 Tailwind） | ^2.8.0 |
| echarts | 图表（不引入 echarts-for-react） | ^5.5.0 |
| lucide-react | 图标 | ^0.460.0 |
| @tauri-apps/api | Tauri invoke 通信 | ^2.2.0 |

### 自定义封装

| 文件 | 用途 |
|------|------|
| hooks/useTauriQuery.ts | 基于 react-query 封装 invoke |
| hooks/useECharts.ts | 原生 echarts hook（useRef + 原生 API） |
| hooks/useGameTheme.ts | 游戏切换时换 CSS 变量 |

## Rust 后端

| Crate | 用途 | 版本锁定 |
|-------|------|----------|
| tauri | 桌面框架核心 | 2.2 |
| rusqlite (bundled) | SQLite 数据库 | 0.32 |
| r2d2 | 连接池（替代裸 Mutex） | 0.8 |
| r2d2_sqlite | r2d2 SQLite 适配 | 0.25 |
| serde (derive) | 序列化 | 1 |
| serde_json | JSON | 1 |
| anyhow | 统一错误类型 | 1 |
| sysinfo | 进程列表扫描 | 0.33 |
| chrono | 日期时间处理 | 0.4 |
| image | 图片加载 + 颜色直方图比较（不用 imageproc） | 0.25 |
| windows | WinRT API：OCR / Toast / 窗口检测 | 0.58 |

### windows crate feature 限定（禁止全量引入）

```toml
windows = { version = "0.58", features = [
    "Media_Ocr",
    "UI_Notifications",
    "UI_WindowAndInput",
] }
```

## 关键代码约定

### Rust 结构体序列化

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaRecord {
    pub id: i64,
    pub item_name: String,    // 前端接收 itemName
    pub star_rating: i32,     // 前端接收 starRating
    pub game_kind: String,    // 前端接收 gameKind
    // ...
}
```

### 数据库操作

```rust
// 所有数据库操作包在 spawn_blocking 中
let pool = app.state::<DbPool>();
let records = tokio::task::spawn_blocking(move || {
    let conn = pool.get()?;
    // ... execute queries
}).await??;
```

### Tauri 命令签名

```rust
#[tauri::command]
fn get_gacha_records(game_kind: String) -> Result<Vec<GachaRecord>, String> {
    // 内部用 anyhow 包装
}
```

## 安全配置

`tauri.conf.json` 中：
- **不开启** `shell.open`（默认关闭所有危险权限）
- 不设置 `allowlist`，使用 capabilities 替代
- CSP 锁定：`default-src 'self'; img-src 'self' asset: https://asset.localhost; style-src 'self' 'unsafe-inline'`

## AI 开发约束（每次提交任务时附带）

```
1. 所有路由使用 HashRouter，不用 BrowserRouter（Tauri 无服务器）
2. 图表用 useRef + echarts 原生初始化，不依赖 echarts-for-react
3. Rust 结构体添加 #[serde(rename_all = "camelCase")]
4. 数据库操作在 spawn_blocking 内执行（rusqlite Connection 非 Send）
5. 数据库连接用 r2d2 连接池，不用裸 Mutex<Connection>
6. windows crate 只开用到的 feature，不用 windows::*
7. tauri.conf.json 不开启 shell.open（默认关闭危险权限）
8. 截图识别用 image 颜色直方图，不引入 imageproc
9. Rust 错误统一用 anyhow::Result
10. 不擅自升降 package.json / Cargo.toml 主版本号
```
