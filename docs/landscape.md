# rm-manager 同类工具调研（Landscape）

**目的**：为产品取舍提供参照——同类工具在数据模型、交互与存储上的优缺点证据，含可借鉴点与反面教训。
**日期**：2026-10-14
**方法**：三路并行调研（服务端型 PM 工具 / 本地优先个人工具 / 需求工程与方法论），基于 2026-10 实时网络搜索核实；网页全文抓取受网络策略限制，个别版本号与维护状态在落地前需抽查官方原文。本文档仅作设计参考，不引入任何代码依赖。

## 一、跨三路结论（已影响本项目的取舍）

1. **单一条目实体 + 类型字段**：OpenProject 的 Work Package、Plane 的 Work Item 都是统一实体按 type 区分；Redmine 用 Tracker 让不同类型走不同状态流——与我们的模型一致
2. **一文件一条的纯文本，是 Git 同步场景的正解**：Obsidian 生态的 TaskNotes 实践相同；反例——Taskwarrior 3.0、Logseq 2.0、AppFlowy、TriliumNext、Anytype 改为专有或加密块存储后，git diff、grep、跨工具迁移全部失效（Logseq 因此流失用户）
3. **捕获必须零决策**：Logseq「默认收件箱＝今天」、Todoist 一次输入解析、Linear 的 Inbox 是唯一 triage 入口；被弃用工具的第一死因是「记录前先要选项目/类型/日期」
4. **默认值给足、配置收好**：Super Productivity 开箱即用（默认项目/视图/计时） vs Taskwarrior「默认太少、全要自己配」的普遍差评
5. **状态可跳转、WIP 只软提醒**：Kanboard 的 WIP 超限只高亮不阻止；Plane 的状态是「分组 + 组内自定义列」而非固定枚举
6. **保存的视图是一等公民**：Taskwarrior 的 report、Obsidian 的 Dataview/Bases、org-mode 的 org-agenda 殊途同归
7. **历史、基线、回滚交给 git**：Doorstop、StrictDoc、TaskJuggler 一致采用「版本控制即基线」——我们不自建
8. **关系先少后多**：ReqIF 的类型化关系系统对个人零收益；Wekan「Checklist + Subtask 双套下级概念并存」是反面教材；Doorstop 把 links（我引用谁）与 derived（我派生自谁）拆开是轻量正解
9. **流程靠默认值与提醒教，不靠强制**：org-mode 的零 schema 自由对新手不友好，但规则堆多更糟——模板与默认值才是引导
10. **不做自动排期**：TaskJuggler 证明个人场景（尤其预研估不准时）自动排程＝垃圾进垃圾出

## 二、工具清单

### 服务端型开源 PM / 需求跟踪

共性：全部需要自托管（多为 Docker 多容器 + 数据库）；成本从 Kanboard/Vikunja 的「单容器 + SQLite（512MB～1GB）到 Plane/OpenProject/Taiga 的「多容器 4GB+」。

- **Plane**（[plane.so](https://plane.so/open-source)）：现代 Jira/Linear 式；状态＝分组 + 组内自定义列、Triage 收件箱为唯一入口、Cycles（时间）/Modules（主题）两条正交归属。借鉴：状态分组、收件箱唯一入口；反面：实用的多级层级被锁进商业版、栈重
- **Vikunja**（[vikunja.io](https://vikunja.io/features/)）：自托管 Todoist（Go + SQLite）；桶（Bucket）+ 容量上限、保存筛选器、Markdown 描述。借鉴：保存筛选器、桶命名直白；反面：无限嵌套＝配置负担
- **Leantime**（[github.com/Leantime/leantime](https://github.com/Leantime/leantime)）：明确「为非项目经理人群」设计；Goal → Milestone → Task、Ideas 想法板、命名避免敏捷黑话。借鉴：显式「想法 →（升格）条目」路径与去黑话命名
- **Kanboard**（[kanboard.org](https://kanboard.org/)）：极简看板（PHP + SQLite）；泳道 × 列正交、WIP 超限只高亮、强查询语法。借鉴：软提醒、查询语法。注意：官方已进入 maintenance mode，Debian 讨论移除——只做参照、不做依赖
- **Wekan**（[wekan.fi](https://wekan.fi/status/)）：老牌看板；反面：Checklist 与 Subtask 两套下级概念并存；发版与合并由单人承担，维护风险高
- **Focalboard**：已 EOL（社区 fork 在寻找维护者）；其「属性 schema 与视图解耦」是最像 Markdown + frontmatter 的模型。仅作只读参考
- **Taiga**（[taiga.io](https://taiga.io/)）：敏捷 Scrum/Kanban；开局选模板的起始屏降低起步成本；Story Points 对单人属过度设计
- **Redmine**（[redmine.org](https://www.redmine.org/)）：「按类型走不同状态流」的正统来源；插件升级地狱是反面
- **OpenProject**（[openproject.org](https://www.openproject.org/)）：统一 Work Package + type（验证我们的实体模型）；Baseline（计划快照对比现状）可服务复盘阶段
- **闭源 UX 参照**（仅交互参考）：Linear——Cmd/Ctrl+K、列表即操作台、Inbox 唯一入口；Todoist——一次输入解析出日期/项目/标签；Things——「今天」作为唯一焦点清单

### 本地优先 / 纯文本 / 个人向

- **Taskwarrior**（[taskwarrior.org](https://taskwarrior.org/docs/)）：最强 CLI 任务管理；默认太少全要自配、sharp edges 多、v3 转 SQLite 后 git merge 基本失效。借鉴：自定义 report＝保存视图、hooks＝自动提交挂载点
- **todo.txt 生态**（[格式规范](https://github.com/todotxt/todo.txt)）：格式 5 分钟学会、一行正则可解析；「归档已完成」一个小机制解决 90% 列表噪音。缺点：无层级、无正文、生态低活跃
- **Obsidian 生态**：markdown 项目管理的事实标准；「一行一条」（Tasks）与「一文件一条」（TaskNotes）两条路线均有实践；插件越堆越重是公认陷阱。借鉴：字段全可空否则捕获变慢；属性 + 视图分工（Dataview/Bases）；obsidian-git 是同类同步实践
- **Logseq**（[logseq.com](https://logseq.com/)）：journal-first，「默认收件箱＝今天」是最大 UX 资产；2.0 转 DB 存储导致纯文本与 git 优势消失、用户流失——正反双重教训
- **AppFlowy**（[appflowy.io](https://appflowy.io/)）：本地优先 Notion 平替；类型化字段 + 多视图（同一数据换视图）是最直观心智模型；但数据非纯文本，git 无意义
- **TriliumNext**（[triliumnext.github.io](https://triliumnext.github.io/Notes/)）：树状层级 + label 继承 + 模板＝轻量类型系统；把数据库当真相，与 git 同步诉求正面冲突
- **SilverBullet**（[silverbullet.md](https://silverbullet.md/)）：frontmatter 约定 + 运行时查询替代独立索引，个人规模极省事；反面：把可编程性当主卖点抬高门槛
- **Anytype**（[anytype.io](https://anytype.io/)）：万物皆对象、对象有 Type——类型心智最完整；加密块存储让 grep/git 全部失效
- **Super Productivity**（[super-productivity.com](https://super-productivity.com/)）：本地优先待办 + 时间盒；默认值给足是范例；数据格式自我封闭是反面

### 需求工程与规划（文本化）

- **Doorstop**（[github.com/doorstop-dev/doorstop](https://github.com/doorstop-dev/doorstop)）：Python CLI，条目落 YAML/Markdown；links/derived 拆分的追溯机制、git 即基线。无 GUI；正交字段对个人偏重
- **StrictDoc**（[strictdoc-project](https://strictdoc.readthedocs.io/en/stable/stable/docs/strictdoc_01_user_guide.html)）：面向安全关键场景；STATEMENT（做什么）/RATIONALE（为什么）分家、带角色的 RELATIONS、追溯矩阵；合规仪式是个人负担
- **ReqIF / Eclipse RMF**（[projects.eclipse.org](https://projects.eclipse.org/projects/modeling.mdt.rmf)）：OMG 标准的类型化关系元模型；为组织互操作付出的类型系统成本，对个人收益为零
- **TaskJuggler**（[taskjuggler.org](https://taskjuggler.org/tj3/manual/Tutorial.html)）：纯文本项目规划、声明式依赖 + 自动排期；单人场景自动排期＝垃圾进垃圾出。「只声明最少信息、让工具算」的思想可取，「算排期」不可取
- **org-mode**（[orgmode.org](https://orgmode.org/manual/TODO-types.html)）：文件即层级、状态序列 + LOGBOOK 时间线、agenda＝跨文件聚合视图、归档＝搬迁；我们「捕获 → 整理 → 视图」的设计与其一一对应；零 schema 自由对新手不友好

## 三、方法论 → 产品默认值

| 方法论 | 落成什么 |
| --- | --- |
| GTD | 收件箱 + 每周回顾，做成默认开启的两个仪式 |
| Shape Up | 「预算」三档（小/中/大）替代精算；团队仪式（pitch/下注）不进工具 |
| 看板 | WIP 超限软提醒（默认 3），不硬拦 |
| MoSCoW | 价值在「不做」有显式归宿：dropped 必填原因、可退回 |
| RICE | 砍掉公式与小数分；只留影响/成本两维 + 一条置信度自问 |
| 2x2 价值-成本 | 做成视图教取舍（P2）；提示「别永远待在快赢格」 |
| 艾森豪威尔 | 每周一问（"这周有重要不紧急的投入吗？"），不做成字段 |

## 四、新手不出坑（调研共识）

1. 先建收件箱与每周回顾，再谈字段
2. 字段能少则少：默认只要求一个标题，其余全可空
3. 关系只从一种起步（父子/链接），需要时再扩展
4. 基线、历史、回滚全交给 git，不自建
5. 让工具「提醒」，不让工具「管束」

**被弃用五模式**：捕获有摩擦 / 字段必填太多 / 维护与配置成本高 / 工具变成项目本身 / 上游停更或格式迁死（Focalboard EOL、Kanboard 维护模式、Logseq 转 DB）。对应本项目的红线：**捕获一次回车、字段全可空、零运维、纯文本存储**。

## 五、明确不采纳的参照

- 自动排期（TaskJuggler 式）——个人估不准，垃圾进垃圾出
- 类型化关系系统（ReqIF 式）——对个人零收益
- 专有 DB / 加密块存储（Anytype / TriliumNext / Logseq 2.0 式）——与 git 同步诉求冲突
- 无限嵌套与自定义字段泛滥（Vikunja / Notion 式）——固定流程，拆解建议 ≤2 层
- Sprint / 迭代概念——个人按周规划，不引入
