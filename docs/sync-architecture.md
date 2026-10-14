# rm-manager 远程同步架构（Git 跨机同步）

**方案**：文件即数据库 + 私有 Git 仓库托管
**日期**：2026-10-14
**适用**：Tauri 桌面应用 · 单用户 · 无私人服务器 · 公司电脑 ↔ 家庭电脑同步

## 1. 结论

数据真源是一堆纯文本文件（一个条目一个 Markdown 文件），本地 SQLite 只做搜索索引（可随时重建、不进仓库）；同步用 HTTPS 协议推拉一个 Gitee/GitHub 私有仓库。不需要任何服务器，免费，且顺带白拿三样东西：跨机同步、完整变更历史、异地备份。

```
          私有 Git 仓库（Gitee / GitHub Private，只存文本文件）
                ▲                              ▲
       git pull/push（HTTPS 443 + PAT）  git pull/push
                │                              │
      ┌─────────┴─────────┐      ┌─────────────┴─────┐
      │ 公司电脑            │      │ 家庭电脑           │
      │ rm 应用（Tauri）    │      │ rm 应用（Tauri）   │
      │  vault/  数据真源   │      │  vault/ 数据真源   │
      │  index.db  本地索引*│      │  index.db 本地索引*│
      └───────────────────┘      └───────────────────┘
         * 可随时删除重建，绝不进 Git —— 冲突面清零
```

## 2. 为什么必须「文件为真源」，而不是直接同步 SQLite

- SQLite 是二进制单文件，git 无法合并。两端都改过 = 整文件二选一，**必然丢数据**。
- 一个条目一个 Markdown 文件：两端改的是不同条目、或同一条目的不同部分时，git 大多能自动合并；只有同一条目同一处被两边改过才冲突——单用户交替使用下这是罕见事件。
- 附加红利：每个条目的变更历史、diff、撤销全部免费获得（git log 就是活动日志）；数据永远不被应用绑架，任何编辑器可读。

## 3. vault 目录结构

```
rm-vault/                    # 独立目录（如 D:\rm-vault），本身就是一个 git 仓库
├─ .git/
├─ .gitattributes            # * text=auto eol=lf  —— 消灭 Windows CRLF 假冲突
├─ items/                    # 所有条目，一文件一条
│    REQ-2026-0001--报表导出重构.md
│    SPIKE-2026-0003--消息队列选型.md
│    IDEA-2026-0011--批量操作的想法.md
├─ decisions/                # 预研结论/决策记录（ADR 风格）
├─ views/                    # 保存的查询视图（yaml）
└─ attachments/              # 附件（P2 再决定大文件是否同步）
```

条目文件格式（frontmatter 存属性，正文是 Markdown）：

```markdown
---
id: REQ-2026-0001
type: requirement        # idea | requirement | bug | spike | chore | session
title: 报表导出重构
status: refining         # inbox|clarifying|refining|ready|scheduled|doing|blocked|review|done|dropped
space: work              # work | personal
project: 内部报表平台
priority: P1
estimate: 3              # 理想人日；空 = 未估
due: 2026-10-30
tags: [报表, 后端]
parent: null             # 拆解子项 / 头脑风暴会话的想法
links: [SPIKE-2026-0003]
created: 2026-10-14T10:00
---

## 背景
...
## 验收标准
- [ ] ...
```

id 前缀与类型固定映射：`IDEA-` 想法 / `REQ-` 需求 / `BUG-` Bug / `SPIKE-` 预研 / `CHORE-` 杂务 / `SESS-` 头脑风暴会话。

## 4. 同步流程与防冲突设计

**同步协议（应用内自动完成）：**

1. **保存即本地 commit**，commit 消息自动生成（如 `REQ-2026-0001: status ready→doing`）。纯本地瞬时操作，强杀进程也不丢内容。正文编辑 30 秒防抖，状态变更等动作级修改立即提交。
2. **启动时 / 每 15 分钟 / 点同步按钮**：`git pull --rebase`（开 autoStash）。有远端更新则重建受影响文件的索引，几乎无感。
3. **推送**：定时或退出时 `git push`。失败（断网/认证）静默重试，状态栏角标提示，不弹窗打断。
4. **冲突**（两端改了同一条目同一处）：rebase 中止 → 应用顶部横幅列出冲突条目 → 三选一解决：「保留双方（远端版存为副本条目）/ 用本地 / 用远端」→ 自动完成 rebase + push。

**让冲突尽量不可能发生，靠的是设计而不是补丁：**

- 一条目一文件 → 改不同条目的操作永不冲突
- 排序尽量派生（按优先级/日期算），不把显式排序数组存进共享文件；视图的本地配置（排序、列宽）只存本机
- 活动日志不落文件——git log 就是
- 高频小改动靠防抖合并 commit，控制历史噪音

## 5. 托管与鉴权（无服务器）

- **首选 Gitee 私有仓库**：国内双线稳定、免费、公司网络一般可达。HTTPS + 私人令牌（最小权限即可）。
- **备选 GitHub 私有仓库**：用 fine-grained PAT（只授权该仓库 Contents 读写）；公司网络可能慢或被策略拦截，必要时配代理。
- **不用 SSH 的理由**：公司网络常封 22 端口；HTTPS 走 443 最不容易被拦（SSH 作为可选支持）。
- **令牌存放**：存 Windows 凭据管理器（keyring），调用 git 时临时注入，不写进 `.git/config` 明文，不动全局 git 配置。
- **空间级远端隔离**（工作+个人兼顾的关键设计）：`work` 和 `personal` 两个空间可以各配一个远端仓库，互不混装；工作空间若有保密顾虑，可以干脆**不配远端、只留本地**（同步开关按空间独立）。
- **合规提醒**：把工作内容推到外部托管属于数据外传，先确认公司政策再连远端。

## 6. 一次性配置（各做一次）

1. Gitee 上新建一个空的私有仓库（不要初始化 README）。
2. 公司电脑：应用里「初始化 vault 并连接远端」→ 首次推送。
3. 家庭电脑：应用里「从远端克隆」→ 得到同样结构。

## 7. Tauri 实现要点

- **git 用「调用系统 git 二进制」而非内嵌库**：两端都是 Windows、装了 Git for Windows 即可；系统 git 自带凭据管理、代理、证书处理，最省事。应用启动检测 `git --version`，缺失给引导。
  `shortcut: 依赖系统 git；升级路线是内嵌 gitoxide，免依赖但认证要自写——等真有非开发用户时再换。`
- Windows 细节：子进程加 `CREATE_NO_WINDOW` 防黑框闪现；路径按 UTF-8 处理；仓库局部设 `core.autocrlf=false` + `.gitattributes` 强制 LF。
- 索引用 rusqlite + FTS5，按文件 mtime+hash 增量重建；vault 路径可配置且建议放在纯本地盘（别放 OneDrive 目录，避免二次同步打架）。
- 桌面形态正好把 Tauri 的全局快捷键（速记）、托盘、系统通知用起来。

## 8. 明确不采用（及原因）

- **直接同步 SQLite / 网盘同步单文件**：二进制冲突 = 丢数据，否决。
- **Syncthing 等 P2P**：不需要服务器，但公司网络普遍封 P2P 打洞，且不满足 git/HTTP 偏好，备注级备选。
- **Cloudflare Workers + R2 自建极简 HTTP 同步**：无固定服务器、免费额度够，但要自己实现差异与冲突处理，复杂度远高于 git；若哪天完全不信任何托管再考虑。
- **最后手段兜底**：万一公司网络连 Gitee 都不可达，家庭机照常推远端，公司机用 U 盘 `git bundle` 导入导出，一周一次也够用。
