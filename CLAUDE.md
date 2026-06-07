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

---

## 技术栈

- **桌面框架**：Tauri 2.x（Rust 后端 + WebView2 前端）
- **前端**：React 18+、TypeScript、Vite、HeroUI（`@heroui/react`）、Tailwind CSS
- **状态/路由**：Zustand + React Router v6（**必须用 HashRouter**）
- **数据请求**：`@tanstack/react-query`，通过 `useTauriQuery`/`useTauriMutate` 调用 invoke
- **包管理**：pnpm
- **数据库**：SQLite（`rusqlite` + `r2d2` 连接池）
- **外部 HTTP**：`reqwest`（blocking + json + native-tls）
- **格式化**：Prettier 全局安装，配置在 `.prettierrc`

## Rust 后端约束

- 结构体标注 `#[derive(Serialize, Deserialize)] #[serde(rename_all = "camelCase")]`
- 数据库操作在 `spawn_blocking` 中调用（`rusqlite::Connection` 非 Send）
- 使用 `r2d2` 连接池，不用 `Mutex<Connection>`
- 连接池加 `.connection_timeout(30s)`，否则连接耗尽永久挂起
- 错误用 `anyhow`，Tauri 命令返回 `Result<T, String>`（`TauriResult<T>`）
- 图片处理仅用 `image` crate（加载、裁剪），不用 `imageproc`

## 常用命令

```bash
pnpm tauri dev                     # 开发模式（Vite HMR + Rust 热重载）
cd src-tauri && cargo check        # Rust 编译检查
cd src-tauri && cargo test         # Rust 测试（含 game 和 gacha 模块单元测试）
cd src-tauri && cargo clippy       # Rust lint
npx tsc --noEmit                   # TypeScript 检查
pnpm build                         # 前端构建
pnpm tauri build                   # 打包成 .msi / .exe
pnpm format                        # Prettier 格式化前端代码
pnpm test                          # Vitest 前端测试
```

## 关键路径

```
frontend/
├── pages/
│   ├── Overview.tsx     # 总览页：统计卡片 + FiveStarReview + banner tab 切换
│   ├── Gacha.tsx        # 抽卡页：Banner tabs + 筛选栏 + RecordTable（仅列表）
│   └── Settings.tsx     # 设置页：游戏配置编辑
├── stores/              # Zustand stores（gameStore）
├── hooks/               # useTauriQuery / useECharts
├── components/
│   ├── Layout.tsx       # 布局 + 导航 + GameSwitch
│   ├── RecordTable.tsx  # 分页表格、编辑、删除、歪/不歪切换
│   ├── FiveStarReview.tsx # 5★ 出卡回顾（卡片列表，替换 LuckChart）
│   └── StatCard.tsx     # 统计卡片
└── lib/                 # 常量、类型（THEMES / ROUTES / GameKind）
src-tauri/src/
├── commands/
│   ├── gacha.rs         # 抽卡命令 + auto_import_gacha_log
│   └── config.rs        # gacha_config.json 读写 + authkey 缓存
├── game/                # GameKind 枚举
├── ocr.rs               # OCR 入口，OnceLock 延迟初始化
├── paddle.rs            # PP-OCRv4 引擎封装
├── error.rs             # TauriResult<T>
└── db.rs                # r2d2 连接池 + 表迁移
```

## Tauri Commands 一览

| 命令 | 说明 |
|------|------|
| `get_gacha_stats` | 返回 GachaStats（totalPulls, byBanner[] 等） |
| `get_gacha_records` | 分页查询，支持 banner/star/sort 过滤 |
| `get_gacha_chart_records` | 全量记录不分页（供 FiveStarReview 使用） |
| `import_gacha_screenshot(s)` | OCR 导入截图 |
| `import_gacha_log` | 自动从游戏日志提取 authkey → 调米哈游 API 拉取 |
| `update_gacha_record` | 编辑单条记录 |
| `delete_gacha_record` | 删除单条记录 |
| `get/save/reset_gacha_config` | 配置读写 |

## 数据流

```
React Component → useTauriQuery (react-query) → invoke → Tauri Command
                                                          ↓
                                                     spawn_blocking
                                                          ↓
                                                    r2d2 pool → SQLite
```

## 关键设计决策

### 抽卡记录自动导入（auto_import_gacha_log）

- **authkey 提取**：从 `Player.log` 中搜索 `auth_appid=webview_gacha` 上下文，提取 authkey 参数
- **authkey 缓存**：首次提取成功后写入 `gacha_config.json`，有效期 24h，后续刷新直接复用
- **API**：调用米哈游 `getGachaLog` 接口，`end_id` 游标分页，每页 20 条
- **时间限制**：45 秒超时，防止卡死
- **去重**：`ON CONFLICT(game_kind, item_name, record_date) DO UPDATE`
- **安全**：authkey 不落日志，只在函数栈内存中存在

### gacha_type 卡池类型映射

**星穹铁道：** 1=常驻, 2=新手, 11=角色活动, 12=光锥活动
**原神：** 100=新手, 200=常驻, 301=角色活动, 302=武器活动, 500=集录祈愿

只有「角色/武器/光锥活动」有 50/50 机制。常驻/新手/集录无歪的概念。

### 歪 / 不歪判定（FiveStarReview）

- **启发式规则**：限定池 5★ 角色名在常驻名单（白露/布洛妮娅/姬子/杰帕德/克拉拉/瓦尔特/彦卿 / 迪卢克/琴/刻晴/莫娜/七七/提纳里/迪希雅）中 → 判定为歪
- **优先使用数据库值**：用户手动在列表页点了「歪了」（isWon=false）后，总览页优先读取数据库值
- **大保底累积**：限定池歪了后下一条限定池出货显示「歪的抽数 + 保底抽数」的合计值

### 欧非 vs 歪不歪（概念分离）

- **欧非评级**：基于花费抽数，所有卡池都有（⚡欧皇/✨欧/✅不错/正常/💀非/💀究极非酋）
- **歪/不歪**：客观事实，仅限定池有（歪了/没歪/大保底），hover 时展示
- 列表页结果列按钮：没歪（绿色）/ 歪了（红色）

### 配置文件（gacha_config.json）

存储在 `AppData/Roaming/com.dakubo.astrorigin/gacha_config.json`，包含：
- `logDirs`：游戏日志目录路径（支持 %USERPROFILE% 等环境变量）
- `apiUrl`：米哈游 API 地址
- `extraParams`：额外 URL 参数（region, game_biz）
- `gachaTypes`：卡池类型代码 → 名称映射
- `authkey` / `authkeyExpiresAt`：缓存 authkey 及其过期时间

### 其他

- **OCR 引擎**：PP-OCRv4（ONNX + tract 纯 Rust 推理），~15MB 模型在 `assets/models/`
- **OCR 初始化**：`OnceLock` 延迟加载，`Mutex<OcrEngine>` 保证线程安全
- **模型路径解析**：3 级 fallback（exe 同级 → CARGO_MANIFEST_DIR → 当前目录）
- **单实例**：`tauri-plugin-single-instance`，二次启动聚焦已有窗口
- **OCR 管线**：整图 OCR → 坐标聚类行列 → 按表头 X 范围分列 → 模糊匹配 → 去重入库
- **宽容策略**：OCR 识别不准时保留空字段入库，用户可编辑修复
- **去重**：UNIQUE 索引 `(game_kind, item_name, record_date)`，`INSERT OR IGNORE`
- **游戏切换**：Zustand store + CSS 变量，不重启应用

## AI 自检清单

1. 路由是否用了 HashRouter？
2. Rust 结构体是否加了 `#[serde(rename_all = "camelCase")]`？
3. 数据库操作是否在 `spawn_blocking` 中执行？连接用 r2d2？
4. tauri.conf.json 是否未开启 shell.open 或其他危险权限？
5. 是否擅自升降了 package.json / Cargo.toml 主版本号？
6. 后端命令注册到 invoke_handler 了吗？
7. 新增的 `get_gacha_chart_records` 是否注册了？（专门给 FiveStarReview 用）
8. 游戏日志文件读取失败时是 `continue` 而不是 `?`（output_log.txt 可能非 UTF-8）
