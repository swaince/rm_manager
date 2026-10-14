# Repository Guidelines

个人需求管理桌面应用：Tauri 2（Rust 后端 + Vue 3 前端）、Windows 优先、Markdown 文件为数据真源、Git 同步。代码骨架已用官方脚手架（create-tauri-app）初始化，提交历史为空（约定见下文）。

## Project Structure & Module Organization

```
GLOSSARY.md  # 领域词汇表（根目录）
src-tauri/   # Rust 后端：vault 文件读写、SQLite 索引（可随时重建）、git 同步
src/         # 前端界面（WebView）
docs/        # 设计文档：feature-list.md（功能清单）、sync-architecture.md（同步架构）、landscape.md（同类工具调研）
```

用户数据（vault，如 `D:\rm-vault`）是独立于本仓库的目录，不得放进项目内。

## Build, Test, and Development Commands

常用命令（包管理统一 pnpm；应用根 = 本仓库，Rust 代码在 `src-tauri/`）：

- `pnpm tauri dev` — 启动桌面应用（开发模式，前后端热更新）
- `pnpm tauri build` — 打包 Windows 安装包
- `pnpm build` — 前端类型检查 + 构建（vue-tsc + vite，产物 `dist/`）
- `pnpm lint` / `pnpm lint:check` — oxlint + ESLint（含「禁止原生表单」规则），`:check` 只查不改
- `pnpm format` / `pnpm format:check` — Prettier 格式化 `src/`（无分号、单引号、100 列）
- `cargo test` / `cargo fmt` / `cargo clippy` — Rust 侧测试与检查（在 `src-tauri/` 下运行）

## Coding Style & Naming Conventions

- Rust：rustfmt 默认（4 空格、snake_case）；前端：Prettier（无分号、单引号、100 列）+ oxlint + ESLint（配置：`.prettierrc.json`、`.oxlintrc.json`、`eslint.config.ts`）
- 条目 id 固定格式 `TYPE-YYYY-NNNN`（如 `REQ-2026-0001`），文件名 `ID--标题.md`（见 `docs/sync-architecture.md`）
- 所有文本文件统一 UTF-8；Windows 下子进程调用注意 `CREATE_NO_WINDOW`

## 前端约定（Frontend）

- 技术栈：Tauri 2 + Vite + Vue 3 + TypeScript + Tailwind CSS 4 + shadcn-vue（reka-vega / neutral）+ VueUse + ai-elements-vue
- 脚手架与规范来源：官方 `create-tauri-app` 初始化；格式化 / lint 规范、通用组件与目录组织方式取自 `E:\CodeRepo\Agents\phoenix-project\web`（只读参考，**禁止改动 phoenix-project 仓库**）
- `src/components/ui/**` 与 `src/components/ai-elements/**` 为生成的 UI 库代码：**禁止手工修改**，并被 lint / 格式化整体排除；需要定制时在其上包 wrapper 组件
- **禁止原生表单控件**（input / select / textarea / button / option / datalist）：一律使用 shadcn-vue 组件或已有封装，由 ESLint 规则强制；日期时间禁止让 Input 退回原生 `type`
- 通用组件族已自 phoenix 原样移植：`src/components/` 下的 `copy / data-table / date-time / drawer / file-upload / form / layout / pagination / query-form`，工具函数 `src/lib/clipboard.ts`；后续修改保持与 phoenix 同风格（同款 shadcn 风格与封装约定），新组件优先在这些封装之上组合
- CSS 三件套（沿用 phoenix）：`src/styles/index.css` 是唯一入口（转发 tailwind + theme）；`theme.css` 放项目自定义主题与层叠（字体栈、滚动条、表单着色、动效降级等，所有手写样式只放这里）；`tailwind.css` 由 shadcn-vue 脚手架生成，**禁止修改**，并已从格式化中排除

## Testing Guidelines

- Rust 单测与源码同文件（`#[cfg(test)]`），集成测试放 `src-tauri/tests/`
- 业务逻辑尽量放 Rust 侧以保证可测；前端不强制单测
- 必测项：frontmatter 解析、vault 文件读写、git 冲突处理
- 命名风格：`parses_frontmatter_with_missing_due`（动作_场景_预期）

## Commit & Pull Request Guidelines

- 目前无提交历史。约定 Conventional Commits + 中文描述：`feat: 条目状态机`、`fix: 冲突后索引重建`、`docs: 更新同步架构`
- 区分两套 git 场景：本代码仓库 vs 用户数据 vault 仓库——后者 commit 由应用自动生成（如 `REQ-2026-0001: status ready→doing`），不要在 vault 里手工提交
- 个人项目主干直推；开 PR 时写明变更点与本地验证方式

## Agent-Specific Instructions

- 功能变更先对照 `docs/feature-list.md` 的 FR 编号，先改文档再改代码
- 涉及存储/同步的改动必须同步更新 `docs/sync-architecture.md`
- AI 功能默认关闭，不得默认开启或自动调用模型
- PAT、API key、vault 数据绝不写入代码或提交物

## Agent skills

### Issue tracker

Issues 存于 GitHub（swaince/rm_manager 的 Issues），技能通过 `gh` CLI 读写。See `docs/agents/issue-tracker.md`.

### Triage labels

沿用默认 5 个规范标签：`needs-triage` / `needs-info` / `ready-for-agent` / `ready-for-human` / `wontfix`。See `docs/agents/triage-labels.md`.

### Domain docs

单上下文（single-context）：根目录 `GLOSSARY.md` + `docs/adr/`，术语与 ADR 由 `/domain-modeling` 按需懒创建。See `docs/agents/domain.md`.
