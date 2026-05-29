# 星原手记（AstroOrigin）

你是《星原手记》（AstroOrigin）的开发助手。这是一个 Windows 11 桌面应用，技术栈与约束如下，你必须严格遵守。

---

## 技术栈

- **桌面框架**：Tauri 2.x（Rust 后端 + WebView2 前端）
- **前端**：React 18+、TypeScript、Vite
- **组件库**：HeroUI（`@heroui/react`），基于 Tailwind CSS
- **图标**：Lucide（`lucide-react`）
- **路由**：React Router v6（**必须用 HashRouter，不能用 BrowserRouter**，因为 Tauri 使用 `file://` 协议）
- **状态管理**：Zustand
- **数据请求**：`@tanstack/react-query`，通过自定义 hooks `useTauriQuery`/`useTauriMutate` 调用 Tauri invoke
- **图表**：`echarts`（直接通过 `useRef` 初始化，**不要使用 `echarts-for-react`**）
- **虚拟列表/网格**：`@tanstack/react-virtual`（`useVirtualizer`，固定图片宽高比，显式设置 `estimateSize`）
- **样式**：Tailwind CSS + HeroUI 内置样式
- **主题**：支持浅色（派梦）/深色（帕姆）切换，使用 HeroUI 的 `dark` 类名
- **包管理**：pnpm

## Rust 后端约束

- 所有传给前端的结构体必须标注 `#[derive(Serialize, Deserialize)] #[serde(rename_all = "camelCase")]`
- 数据库使用 `rusqlite`，所有同步数据库操作必须封装在 `tokio::task::spawn_blocking` 中调用
- `windows-rs`（`windows` crate）只启用需要的 features，避免全量引入，features 列表写在注释中
  - `Media_Ocr`、`UI_Notifications`、`UI_WindowAndInput`
- 使用 `anyhow` 处理错误，Tauri 命令返回 `Result<T, String>` 或自定义错误类型
- 模板匹配/图片识别使用 `image` + `imageproc`，若可简化则优先裁剪特征区比较颜色直方图
- 进程列表扫描使用 `sysinfo`
- 日期时间使用 `chrono`

## 前端代码规范

- 所有组件使用 TypeScript，明确 Props 类型
- HeroUI 组件优先使用，避免原生 HTML 堆砌
- 图表使用 `echarts.init(domRef.current)` 初始化，在 `useEffect` 中绑定，return 清理函数
- 图片网格/列表必须使用虚拟化，不可直接 map 渲染超过 50 张图片
- 所有路由使用 `HashRouter`，示例：

  ```tsx
  import { HashRouter, Routes, Route } from 'react-router-dom';
  ```

- 使用 Zustand 管理全局状态（如当前游戏、筛选条件），不使用 prop drilling
- 数据请求使用 `useQuery` / `useMutation` 的封装 hooks，不直接调用 `invoke`
- 暗色模式通过 `document.documentElement.classList.toggle('dark')` 或 HeroUI 的 `useTheme` 切换

## 功能上下文

该工具主要服务《原神》和《崩坏：星穹铁道》玩家，核心功能：

- **抽卡记录截图 OCR 解析与本地战绩库**（MVP 最优先）
- **游戏时长与活跃度统计**
- **截图智能策展**（自动标签、场景识别、搜索）
- 体力/派遣循环提醒（后续迭代）

## 设计参考

- 配色方案：派梦（原神吉祥物）暖白红金 + 帕姆（星铁列车长）深蓝红金
- 游戏切换时 UI 主色自动切换（CSS 变量）
- 详细设计见 [docs/superpowers/specs/2026-05-29-genshin-starrail-companion-design.md](docs/superpowers/specs/2026-05-29-genshin-starrail-companion-design.md)
- 技术栈详情见 [docs/tech-stack.md](docs/tech-stack.md)

## AI 自检清单（每次生成代码后检查）

1. 路由是否用了 HashRouter？（Tauri 不允许 BrowserRouter）
2. 图表是否用了 echarts-for-react？→ 改为 `useRef` + 原生初始化
3. Rust 结构体是否加了 `#[serde(rename_all = "camelCase")]`？
4. 数据库操作是否包在 `spawn_blocking` 中？
5. 数据库连接是否用 r2d2 连接池？不直接用 `Mutex<Connection>`
6. windows crate 是否只开了需要的 features？
7. tauri.conf.json 是否未开启 shell.open 或其他危险权限？
8. 图片列表是否超过 50 张？→ 必须用虚拟化
9. Rust 错误是否用 anyhow？返回前转换成 `String`
10. 是否擅自升降了 package.json / Cargo.toml 主版本号？
