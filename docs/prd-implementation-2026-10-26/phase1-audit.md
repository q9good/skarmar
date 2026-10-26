# PRD 0.5 实现符合性审计报告（Phase 1）

- 审计基线：分支 feat/tencent-informed-web；HEAD 3ce27f9eb2d08b660b390bb6a3adb8d57f38324e（2026-10-25 10:18:07 UTC，feat: implement family intervention PRD workflows）
- implementationCommit：491c5bf52ec0ce4746adab4345a4d46f90e6e6ac（2026-10-24）
- PRD 基线：docs/prd-family-intervention.md，版本 0.5
- 审计日期：2026-10-25
- 审计方式：逐条读取 PRD、实施记录、第三轮验证任务、交接状态；读取当前源码和测试；实跑可运行的构建／测试；未修改源码、测试、PRD 或配置
- 重要结论：handoff.md 与 workspace-state.json 仍写 T44=todo，但当前 HEAD 已新增约 1.7 万行 foundation 实现和大量测试。以下结论以实际 HEAD 为准，不沿用该待办状态。
- 总体判定：R01–R16 为 DONE 9、PARTIAL 6、DEVIATION 1；59 条正式 AC 为 DONE 57、PARTIAL 2。Rust 工作区测试通过；TypeScript、Web 构建、浏览器及 DO 合约因当前机器缺依赖或解释器不匹配未完成本轮实跑。

## 一、现状基线（实跑结果）

### 构建与类型检查

package.json 提供的命令如下：

| 脚本 | 命令 |
| --- | --- |
| web | npm run web --workspace apps/client |
| web:build | npm run build --workspace apps/client |
| typecheck | npm run typecheck --workspace apps/client |
| cf:dev | bash tools/start-cloudflare.sh |
| cf:check | wrangler deploy --dry-run --outdir .local/worker-dry-run |
| test:runtimes | node tools/check-runtimes.mjs |
| test:host | node tools/check-host.mjs |
| test:planning | node --test tools/test-planning.mjs |
| test:sync | node --test tools/test-workspace-sync.mjs |
| sync:status／pull／push | node tools/workspace-sync.mjs 相应子命令 |
| test:container | node tools/check-container.mjs |
| test:workflow | node tools/test-workflow.mjs |
| test:relations | node tools/check-relations.mjs |
| test:demo-restore | node tools/check-relations.mjs --restore-demo |
| test:lesson-links | node tools/check-relations.mjs --lessons |
| test:mutations | node tools/check-relations.mjs --mutations |
| test:prd-client | node tools/test-prd-client.mjs |
| test:prd-web | node tools/check-prd-web.mjs |
| test:prd-runtimes | node tools/check-prd-runtimes.mjs |

### 测试运行结果

| 命令 | 结果 | 关键输出／原因 | 耗时 |
| --- | --- | --- | --- |
| npm run typecheck | FAIL（环境） | tsc: not found；仓库当前没有可用 node_modules | <1s |
| EXPO_NO_TELEMETRY=1 npm run web:build | FAIL（环境） | node_modules/.bin/expo 不存在，spawnSync ENOENT | 1s |
| cargo test -p skarma-core | PASS | 3 passed；配置显式选择、拒绝静默默认等测试通过 | 37s（含首次编译） |
| cargo test -p skarma-api | PASS | 136 passed；含 foundation、课程、干预、跟进、观察、导入、认证等集成测试 | 13s |
| cargo test --workspace | PASS | API 136 + core 3 + worker 9，共 148 个测试通过；0 failed | 16s |
| npm run test:planning | FAIL（环境／脚本兼容） | Node 22 直接导入 apps/client/src/planning.ts，ERR_UNKNOWN_FILE_EXTENSION | <1s |
| npm run test:workflow | FAIL（环境／脚本兼容） | Node 22 直接导入 workflow.ts，ERR_UNKNOWN_FILE_EXTENSION | <1s |
| npm run test:sync | PASS | 14 passed，0 failed | 5s |
| npm run test:host | FAIL（环境） | 最终运行 smoke_auth_web.py 时缺少 Python playwright | 26s |
| npm run test:mutations | FAIL（环境） | 缺少 miniflare 包 | <1s |
| npm run test:relations | FAIL（环境） | 缺少 miniflare 包 | <1s |
| npm run test:lesson-links | FAIL（环境） | 缺少 miniflare 包 | <1s |
| npm run test:prd-client | FAIL（环境／脚本兼容） | Node 22 直接导入 foundation.ts，ERR_UNKNOWN_FILE_EXTENSION | 1s |
| npm run test:prd-web | FAIL（环境／脚本） | check-prd-web.mjs 固定调用 python；本机 python 为 Python 2，首先在中文源码处报编码 SyntaxError |
| python3 tools/smoke_prd_foundation_web.py | FAIL（环境） | ModuleNotFoundError: playwright | <1s |
| python3 tools/smoke_page_workflows.py | FAIL（环境） | ModuleNotFoundError: playwright | <1s |
| npm run test:prd-runtimes | FAIL（环境） | 缺少 miniflare，未能执行真实 Rust Worker／DO 对账 | 1s |
| python3 -m py_compile tools/browser_navigation.py tools/smoke_prd_*.py tools/smoke_page_workflows.py | PASS | 全部 Python 3 语法编译通过 | 3s |

说明：

1. 本轮没有安装依赖，因为 Phase 1 约束为只读审计；未为了让测试通过而改变环境或代码。
2. Rust SQLite 集成测试是本轮真实通过结果；浏览器、TypeScript、Web 构建和 Miniflare／DO 均不能沿用历史实施记录冒充本轮通过。
3. tools/check-prd-web.mjs:20 使用固定的 python，而不是 python3；即使改用 python3，当前机器仍缺 Playwright。
4. 测试后 git status --short 为空，工作树保持干净。

### 5eaa910..HEAD 代码改动概览

该范围内 apps／crates 共有 2 个提交：

- 491c5bf：手机交互基础切片。
- 3ce27f9：实现 family intervention PRD workflows。

git diff --stat 结果：113 个 apps／crates 文件，约 +17,000／-207。分区如下：

| 目录 | 文件数 | 新增 | 删除 | 主要内容 |
| --- | ---: | ---: | ---: | --- |
| apps/client | 65 | 3,216 | 154 | 新模型工作区、课记／干预／跟进／统计／离线队列／多草稿／图标等 |
| crates/api | 12 | 4,579 | 14 | 新模型原生 SQLite 路由与 7 组 foundation 集成测试 |
| crates/core | 27 | 8,325 | 6 | foundation v1–v6 schema、领域模型、课记、干预、复盘、统计、转换 |
| crates/http | 6 | 870 | 27 | 契约门禁、PRD 路由、导入导出 |
| crates/worker | 1 | 10 | 6 | PRD 模型初始化与共用事务执行 |

3ce27f9 单独占 104 个文本文件、约 +16,800／-111，另新增 Web 图标资源。该提交显著超出 handoff 中“T44 待实施”的描述。

## 二、逐 R 符合性（R01–R16）

### 批次 A0：基础

#### R14 保存、共享、弱网与权限 — PARTIAL

已实现：

- 账号＋空间＋类型＋记录 ID 的多草稿槽，且有多窗口锁、冲突备份：apps/client/src/foundation-view.tsx:107-166、apps/client/src/draft-store.ts:8-60、apps/client/src/draft-store-native.ts:5-8。
- 基础课记持久离线队列；目录依赖先上传；稳定 ID／operation_id；账号、空间、契约、停用选项、未知回执均暂停：apps/client/src/prd-outbox.ts:46-141、apps/client/src/prd-outbox-view.tsx:26-73。
- 服务端 queued write 再检查已认证 actor／space／契约：crates/http/src/lib.rs:435-468、tools/prd-outbox-contract.mjs:3-23。
- API 契约门禁在反序列化／业务写入前拒绝：crates/http/src/lib.rs:408-432、471-530、552-572。
- 编辑时候选列表固定，后台更新只提示主动核对：apps/client/src/stable-options.tsx:6-25。
- 覆盖层 history 哨兵、键盘／visualViewport 处理：apps/client/src/overlay.tsx:6-85。
- 可信设备 365 天、可撤销服务端 Cookie 会话：crates/core/src/auth.rs:14-15、crates/api/tests/auth.rs:1023-1074。

缺口／偏差：

- PRD 要求“到期前 14 天”提示；客户端对可信设备只在 7 天内提示：apps/client/src/auth.tsx:66-67。
- 尚未尝试上传的本机课记应可取消上传；LessonOutbox 只允许 forget 已确认回执，队列 UI 没有取消 queued 项：apps/client/src/prd-outbox.ts:87-91、apps/client/src/prd-outbox-view.tsx:62-73。
- DO 双运行环境合约存在，但本轮因缺 Miniflare 未实跑。

#### R15 通用引导、控件与用词 — DEVIATION

已实现：

- 集中文案矩阵 63 类字段及三层引导：apps/client/src/field-guidance.ts:2-351、apps/client/src/guided-field.tsx:6-10、apps/client/src/input-guidance.tsx:4-7。
- Web 的日期日历／数字模式互斥并保留未完成输入：apps/client/src/controls.web.tsx:15-32。
- 16／17／14px、44px 热区、大字 18px／56px：apps/client/src/readability.tsx:26-54。
- 固定表单 footer、键盘避让和安全区：apps/client/src/overlay.tsx:70-83。
- 核心颜色静态计算对比度均大于 4.5:1（正文／辅助／placeholder／主按钮）。

偏差：

- PRD 明确“不常驻手动刷新主按钮”，但新模型每个业务页页头常驻“刷新”：apps/client/src/foundation-view.tsx:233。
- 原生 controls.tsx 的 DateField 仍只是一个文本框，不具备日历／数字切换；Web 已实现：apps/client/src/controls.tsx:12-13。
- AC-R15-03 所需“200%文本缩放且键盘开启”本轮没有真实浏览器／真机证据；现有历史脚本主要是页面 zoom 模拟。

### 批次 A1：今天、课记与观察

#### R01 今天 — PARTIAL

已实现：

- 紧凑长期方向一行，最多 3 个名称和“另有 N 项”：apps/client/src/foundation-view.tsx:225、241-243。
- 今天页两个主入口；多个当天计划显式选择：apps/client/src/prd-lesson-view.tsx:228-244、apps/client/src/prd-session-view.tsx:74-92。
- 瞬间观察单列且不计干预：apps/client/src/prd-observation-view.tsx:61-65。
- “给我的待办”合并复盘、课记查看请求和行动，其他家人事项只计数：apps/client/src/prd-followup-view.tsx:137-144。
- 长期详情顺序为意义、最近家庭／课堂观察、关联知识：apps/client/src/foundation-view.tsx:296-303。

缺口：

- 标准页头要求“页头和日期”，当前新模型今天页只显示标题“今天”，没有业务日期：apps/client/src/foundation-view.tsx:230-243。
- “今天已记录”要求作者、时间、“刚刚记录”；现有课程卡显示作者但不显示创建时间，家庭记录显示时段＋作者：apps/client/src/prd-lesson-view.tsx:311、apps/client/src/prd-session-view.tsx:83。
- 页头常驻刷新同时违反 R15。

#### R04 记录上课 — PARTIAL

已实现：

- 今天／昨天／其他日期、课程老师独立选择、搜索新增、各 4 个独立快捷项：apps/client/src/prd-lesson-view.tsx:39-65、268-275、303-309；排序算法在 apps/client/src/prd-lessons.ts:57-65。
- 基础提交不要求观察；重复同日同课同老师只提示不阻止；成功卡复用原 ID 补观察：apps/client/src/prd-lesson-view.tsx:231-232、262-275。
- 服务器稳定目录快照、未来日期拒绝、operation_id 幂等、同 ID 不加节：crates/core/src/foundation/lessons.rs:141-244。
- 创建者创建业务日撤销、引用检查、管理员删除、记错标记分开：crates/core/src/foundation/lessons.rs:478-517。
- Rust 对应测试本轮通过：crates/api/tests/foundation_lessons.rs:159、280、404。

缺口：

- 客户端只按“创建者＋创建当天”显示撤销按钮，不知道入站引用；已被引用的记录仍会显示撤销并在提交后由服务端报冲突，而 AC-R04-05 要求该场景普通成员只能标记记错：apps/client/src/prd-lesson-view.tsx:297-299。
- 离线队列提交后尚未发送的本机课记没有取消上传入口，见 R14。

#### R05 课堂观察与家人接手 — DONE

- 自由文本先于四主题；折叠不清空；输入法语音提示；原因猜测分开：apps/client/src/prd-lesson-view.tsx:67-85。
- 观察／整理分区各自版本，同区冲突、跨区独立：crates/core/src/foundation/lesson_model.rs:158-177、crates/core/src/foundation/lessons.rs:169-181。
- 整理时才明确关联目标／方法／长期方向并产生 0–N 知识、行动、反馈；不从老师建议自动建任务：crates/core/src/foundation/lessons.rs:246-476。
- 待处理／认领／接手／完成与“看到了”分离：crates/core/src/foundation/lessons.rs:310-350、519-526。
- Rust 相关测试本轮通过：crates/api/tests/foundation_lessons.rs:109、212、343、469；crates/api/tests/foundation_completion.rs:102。

#### R16 显示名称 — DONE

- Expo 名称／Web 元数据：apps/client/app.json:1-8。
- 页头和登录显示 skarmar：apps/client/src/foundation-view.tsx:230、apps/client/src/auth.tsx。
- manifest 与 192／512 图标：apps/client/public/manifest.webmanifest:1。
- slug 仍为 skarma；wrangler.toml 中 Worker skarma-web、SkarmaSpace、default 空间均未更名；域名约束没有被代码改动。

### 批次 A2：计划、总结、目标与核心统计

#### R02 干预计划 — DONE

- 默认日期／时段、建议最多 3 项、频率比率排序、暂停／不设排除：crates/core/src/foundation/coverage.rs:196-279、apps/client/src/prd-session-view.tsx:104-117。
- 选中目标立即显示未解决困难、经验和下一步：apps/client/src/prd-session-view.tsx:110-114、apps/client/src/plan-context-view.tsx。
- 主／副／复习互换保留目标笔记；快捷目标／方法创建；计划方法不当成实际方法：apps/client/src/prd-session-view.tsx:108-117、161-163。
- 下一步必须主动纳入并保留版本，不能并发占用：crates/core/src/foundation/sessions.rs:420-438、467-527。
- 首次尝试提示只依据显式 first_try_ids：apps/client/src/prd-session-view.tsx:123-124。
- 无直接 Rust AC-R02-02 专项测试，但排序公式与 tie-break 源码与 PRD 一致；客户端相关测试存在于 tools/test-prd-client.mjs:239-248，本轮因 Node 22 TS 导入失败未运行。

#### R03 干预总结 — PARTIAL

已实现：

- 同 ID 计划／总结；直接总结；至少一项实际目标＋一句表现；未做目标不计：crates/core/src/foundation/sessions.rs:290-349。
- 实际方法 used／none／unknown 严格区分；实际方法版本快照；名称快捷新增并继承实际目标：crates/core/src/foundation/session_model.rs:33-92、apps/client/src/prd-session-view.tsx:118-124、142、162-163。
- 实际标签、情境、8 个参与信号和明确关联范围：crates/core/src/foundation/sessions.rs:373-415。
- 相关困难／经验反馈、0–N 新知识和行动、父子事务、幂等修订：apps/client/src/prd-session-view.tsx:125-140、crates/core/src/foundation/sessions.rs:453-724。
- 共享总结草稿不计次数，正式提交回显原文：apps/client/src/prd-session-view.tsx:71、93-99。

缺口：

- 总结中“新增困难”只有一段 content 和关联，不能在同页分别结构化保存成人回应、原因猜测、条件；这些字段只在后续知识编辑的 Details 中存在：crates/core/src/foundation/lesson_model.rs:51-63、apps/client/src/prd-session-view.tsx:130-132，对照 apps/client/src/prd-followup-view.tsx:177-186。
- 因而 R03 关于“困难描述包括成人回应、观察／原话／推测分清”的即时沉淀只靠自由文本约定，非完整结构化实现。

#### R06 目标 — PARTIAL

已实现：

- 名称＋长期／近期即可保存，默认状态、层级、父级、标准模板、关注节奏、归档：crates/core/src/foundation/model.rs:4-58、apps/client/src/foundation-view.tsx:47-71。
- 指标、帮助条件、指标版本、测量与来源快照独立保存；机会 0 不算伪造进展、做到数不能大于机会数：crates/core/src/foundation/observation_model.rs:40-72、crates/core/src/foundation/observations.rs:192-240。
- 覆盖与进展在目标页分开：apps/client/src/foundation-view.tsx:250-263。
- Rust 测试本轮通过：crates/api/tests/foundation_observations.rs:52、96；crates/api/tests/foundation_completion.rs:346。

缺口：

- 新 GoalInput 没有 PRD 字段矩阵要求的 goal.domain 和 goal.due；前端“更多设置”也没有领域标签或目标日期：crates/core/src/foundation/model.rs:29-53、apps/client/src/foundation-view.tsx:55-70。
- 基础 GoalInput 的帮助条件也不独立，只有后续观察指标中可补。

#### R13 统一统计口径 — DONE

- Asia/Shanghai 含端点窗口、未来／草稿／删除／撤销排除：crates/core/src/foundation/coverage.rs:94-178。
- 家庭／机构／合计分量、快照父级去重、最近日期、方法 unknown／empty、标签、热力、固定色阶：crates/core/src/foundation/coverage.rs:179-324。
- 瞬间观察不进入 coverage；目标进展只用测量：crates/core/src/foundation/observations.rs、apps/client/src/prd-coverage.ts:4-26。
- 附录 D 金标准 Rust 测试本轮通过：crates/api/tests/foundation_sessions.rs:382。
- AC-R13-02／03 由同一测试及 crates/api/tests/foundation_observations.rs:96 覆盖。

### 批次 B：方法、知识和复盘

#### R07 方法 — PARTIAL

已实现：

- 多目标关联、状态、置顶／需注意、版本和来源依据快照：crates/core/src/foundation/model.rs:60-111、crates/core/src/foundation/mod.rs:310-412。
- 排序优先级与原因说明：apps/client/src/prd-history.ts:21-29、apps/client/src/foundation-view.tsx:265-277。
- 家庭／机构实际使用数、最近使用、版本分列：apps/client/src/method-usage.ts:5-12、apps/client/src/method-usage-view.tsx:9-11。
- 就地心得自动关联当前方法：apps/client/src/foundation-view.tsx:335、apps/client/src/prd-followup-view.tsx:76。
- 历史实践保留当时版本：crates/api/tests/foundation_sessions.rs:289（本轮通过）。

缺口：

- PRD 要求正常方法至少有名称、具体做法、明确目标；当前服务端和普通“新建方法”页面允许 description／steps 全空，只校验名称和目标。名称-only 例外本应限于计划／总结快捷创建：crates/core/src/foundation/model.rs:69-90、crates/core/src/foundation/mod.rs:310-338。
- method.observe 和 method.scope 未按字段矩阵独立建模；当前把“适用条件与步骤”合为 steps，缺少单独观察意图：apps/client/src/foundation-view.tsx:73-87。

#### R10 跟进／经验 — DONE

- 经验阶段、状态、关键标记、多目标／方法关系及派生关系：crates/core/src/foundation/lesson_model.rs:42-63、crates/core/src/foundation/process_model.rs:43-64。
- 三类等权反馈；同经验同来源唯一；显式修订移动分类但总数不增；其他家人附注不改主结果：crates/core/src/foundation/followups.rs:423-667。
- 最近相反观察保留；阶段不自动升级：apps/client/src/prd-followup-view.tsx:162-169。
- Rust 对应测试本轮通过：crates/api/tests/foundation_followups.rs:95；crates/api/tests/foundation_completion.rs:501。

#### R11 跟进／困难 — DONE

- 结构化期望、观察、成人回应、猜测、条件、比较口径；正式保存要求关联：crates/core/src/foundation/process_model.rs:6-35、crates/core/src/foundation/followups.rs:92-250。
- 同口径、同帮助条件、比较期和方法版本边界：crates/core/src/foundation/comparison.rs、crates/core/src/foundation/followups.rs:620-667。
- 3 条且最近无变化／更困难时只 offer weekly，不自动提优先级或训练量：crates/api/tests/foundation_followups.rs:147-176（本轮通过）。
- 解决、暂停、恢复、提炼经验和历史均已实现于 followups／reviews。

#### R12 周末复盘 — DONE

- 每对象一个安排、周／隔周／真实日历月算法、手动日期、暂停恢复：crates/core/src/foundation/reviews.rs:28-235。
- 共享去重队列、当前关系和名字、证据分页、已看版本：crates/core/src/foundation/reviews.rs:737-973。
- 决定＋对象状态／方法版本＋下一步＋下一安排原子提交：crates/core/src/foundation/reviews.rs:355-731。
- 参与信号只按明确目标／方法范围显示：crates/api/tests/foundation_followups.rs:302-329。
- 专业更新预览、删减、复制／下载且不自动发送：apps/client/src/prd-followup-view.tsx:159、241。
- Rust 对应测试本轮通过：crates/api/tests/foundation_followups.rs:59、179、302、331、359；crates/api/tests/foundation_completion.rs:396。

### 批次 C：回顾、结算和丰富统计

#### R08 机构课程 — DONE

- 课堂记录／课时核对分段、月步进、自定义区间折叠、按稳定老师 ID 汇总、待核对单列：apps/client/src/prd-lesson-view.tsx:218-258、apps/client/src/prd-lessons.ts:47-55。
- 搜索结构化观察、老师建议、关联知识；“只看老师建议”只取主题字段：apps/client/src/prd-lesson-view.tsx:222、255-258。
- 稳定课程／老师 ID、改名保留快照、停用不改历史：crates/core/src/foundation/model.rs:113-130、crates/core/src/foundation/lessons.rs:200-213。
- PRD Excel 导出接口存在：crates/http/src/lib.rs:81-106、crates/http/src/prd_export.rs。
- Rust 计数和快照测试本轮通过：crates/api/tests/foundation_lessons.rs:159；附录 D 测试通过。

#### R09 家庭干预历史 — DONE

- 自然月热力格、完成数与待总结分开、至少 44px、未来不计完成：apps/client/src/intervention-calendar.tsx:4-9、apps/client/src/prd-session-view.tsx:84-90。
- 近 90 天 13 周整条可点；宽屏才显示日图；最近 30 次显示实际起止：apps/client/src/prd-history-view.tsx:14-30。
- 瞬间单列且不进入热力：apps/client/src/prd-observation-view.tsx:64-65。
- AC-R09-01 的附录 D 热力由 crates/api/tests/foundation_sessions.rs:382 覆盖并本轮通过。

## 三、逐 AC 符合性（附录 C 59 条）

说明：表中“Rust PASS”指本轮 cargo test --workspace 的真实结果；“浏览器未跑”指脚本存在，但本轮缺 Playwright。源码足以确认的 AC 可判 DONE，但不会把未运行脚本写成通过。

### A1 AC

| AC 编号 | 所属 R | 批次 | 状态 | 证据（文件:行号／测试） | 缺口说明 |
| --- | --- | --- | --- | --- | --- |
| AC-R01-01 | R01 | A1 | DONE | foundation-view.tsx:241-245；prd-session-view.tsx:78-83；smoke_prd_sessions_web.py:11-26（浏览器未跑） | 0／3／6 专项脚本仅旧模型版本；新模型由单行 slice(0,3) 源码确认 |
| AC-R01-02 | R01 | A1 | DONE | foundation-view.tsx:296-303 | 意义→最近观察→知识顺序正确；无自动化实跑 |
| AC-R01-03 | R01 | A1 | DONE | prd-session-view.tsx:74-92 | 多个计划打开选择层，不修改计划；浏览器未跑 |
| AC-R04-01 | R04 | A1 | DONE | prd-lesson-view.tsx:39-65、268-275；prd-lessons.ts:57-65 | 快捷项直接选择可满足四次点击；客户端测试未跑 |
| AC-R04-02 | R04 | A1 | DONE | prd-lesson-view.tsx:271-273 | 两字段状态独立 |
| AC-R04-03 | R04 | A1 | DONE | prd-lesson-view.tsx:231-232；lessons.rs:192-244 | 补观察复用原 ID；Rust PASS |
| AC-R04-04 | R04 | A1 | DONE | lessons.rs:478-499；foundation_lessons.rs:280、404 | Rust PASS |
| AC-R04-05 | R04 | A1 | PARTIAL | lessons.rs:490-517 正确拒绝；prd-lesson-view.tsx:297-299 | 服务端正确，但已引用课记客户端仍显示“撤销”，未做到 UI 只给记错入口 |
| AC-R05-01 | R05 | A1 | DONE | prd-lesson-view.tsx:67-80 | 自由文本在四主题之前 |
| AC-R05-02 | R05 | A1 | DONE | prd-lesson-view.tsx:73-81 | 折叠仅改 open，内容不清空 |
| AC-R05-03 | R05 | A1 | DONE | lessons.rs:169-181；foundation_lessons.rs:109 | Rust PASS |
| AC-R05-04 | R05 | A1 | DONE | lessons.rs:170-181；foundation_lessons.rs:109 | Rust PASS，第二次同区写 409 |
| AC-R05-05 | R05 | A1 | DONE | lessons.rs:246-476；foundation_lessons.rs:469 | 只有显式 new_actions 才创建；Rust PASS |
| AC-R16-01 | R16 | A1 | DONE | app.json:3,8；manifest.webmanifest:1；foundation-view.tsx:230 | 浏览器未跑 |
| AC-R16-02 | R16 | A1 | DONE | app.json:4 保留 slug skarma；wrangler.toml:1,21,27 | 技术标识未随显示名改动 |

### A2 AC

| AC 编号 | 所属 R | 批次 | 状态 | 证据（文件:行号／测试） | 缺口说明 |
| --- | --- | --- | --- | --- | --- |
| AC-R02-01 | R02 | A2 | DONE | prd-session-view.tsx:78-80、104-117、93-97 | 无 pending 时 入口→目标→保存，共三次点击＋输入 |
| AC-R02-02 | R02 | A2 | DONE | coverage.rs:222-279 | 按 ratio 降序，再日期、ID；无专项自动化实跑 |
| AC-R02-03 | R02 | A2 | DONE | coverage.rs:222-247 | frequency none／paused 的 ratio 为 null |
| AC-R02-04 | R02 | A2 | DONE | prd-session-view.tsx:110-114；plan-context-view.tsx | 目标选中后知识立即显示 |
| AC-R03-01 | R03 | A2 | DONE | sessions.rs:290-349；prd-session-view.tsx:92、118-124 | Rust PASS：foundation_sessions.rs:88 |
| AC-R03-02 | R03 | A2 | DONE | session_model.rs:41-49；coverage.rs:123-127、316-319 | unknown 与 empty 分列；附录 D Rust PASS |
| AC-R03-03 | R03 | A2 | DONE | sessions.rs:311-415 | 标签／情境／信号均非必填；Rust PASS |
| AC-R03-04 | R03 | A2 | DONE | sessions.rs:453-724；foundation_sessions.rs:136 | Rust PASS，operation_id 幂等且 0–N 输出原子 |
| AC-R03-05 | R03 | A2 | DONE | sessions.rs:311-334 | 只从 performed 目标生成 actual_targets |
| AC-R03-06 | R03 | A2 | DONE | prd-session-view.tsx:142、162-163；foundation.rs:300 | 快捷方法自动关联实际目标并选入；Rust PASS，浏览器未跑 |
| AC-R06-01 | R06 | A2 | DONE | model.rs:31-53；foundation.rs:88 | 仅 title＋level 可保存；Rust PASS |
| AC-R06-02 | R06 | A2 | DONE | foundation-view.tsx:57-60 | 模板和 criteria 分开，切模板不改文字 |
| AC-R06-03 | R06 | A2 | DONE | observations.rs:192-240；foundation_observations.rs:52 | Rust PASS |
| AC-R13-01 | R13 | A2/C | DONE | foundation_sessions.rs:382 | 附录 D 全量 Rust PASS |
| AC-R13-02 | R13 | A2/C | DONE | foundation_observations.rs:96；coverage.rs:108-177 | 瞬间不进入实践集合；Rust PASS |
| AC-R13-03 | R13 | A2/C | DONE | coverage.rs:179-260；foundation_sessions.rs:382 | 一来源以集合去重；Rust PASS |

### B AC

| AC 编号 | 所属 R | 批次 | 状态 | 证据（文件:行号／测试） | 缺口说明 |
| --- | --- | --- | --- | --- | --- |
| AC-R07-01 | R07 | B | DONE | foundation-view.tsx:335；prd-followup-view.tsx:76 | 当前 method_id 自动带入 |
| AC-R07-02 | R07 | B | DONE | foundation_sessions.rs:289；foundation.rs:300 | Rust PASS，实践保存方法快照 |
| AC-R07-03 | R07 | B | DONE | prd-history.ts:21-29；foundation-view.tsx:224-226 | rank 4 在普通 active 后 |
| AC-R10-01 | R10 | B | DONE | followups.rs:423-454；foundation_followups.rs:95-135 | Rust PASS，附注不改总数 |
| AC-R10-02 | R10 | B | DONE | followups.rs:454-667；foundation_followups.rs:112-128 | Rust PASS，分类移动、总数不增 |
| AC-R10-03 | R10 | B | DONE | lesson_model.rs:42-63；prd-followup-view.tsx:177-181 | 可直接选 helpful，无需虚构事件 |
| AC-R11-01 | R11 | B | DONE | followups.rs:654-661；foundation_followups.rs:147-159 | Rust PASS，前两条 offer=false |
| AC-R11-02 | R11 | B | DONE | foundation_followups.rs:147-176；prd-followup-view.tsx:165-167 | Rust PASS，只提示每周回看 |
| AC-R12-01 | R12 | B | DONE | reviews.rs:50-99；foundation_followups.rs:59-75 | Rust PASS，结果 2026-11-21 |
| AC-R12-02 | R12 | B | DONE | foundation_completion.rs:396-427；followups_v4.sql:6-7 | Rust PASS，同方法只有一个 schedule |
| AC-R12-03 | R12 | B | DONE | foundation_followups.rs:359-389；sessions.rs:420-527 | Rust PASS；UI 路线在 smoke_prd_gap_fixes_web.py（未跑） |
| AC-R12-04 | R12 | B | DONE | foundation_followups.rs:179-208；reviews.rs:355-554 | Rust PASS，安排代次和唯一键阻止双完成 |
| AC-R12-05 | R12 | B | DONE | foundation_followups.rs:302-329；reviews.rs:837-884 | Rust PASS，只列明确关联信号 |

### C AC

| AC 编号 | 所属 R | 批次 | 状态 | 证据（文件:行号／测试） | 缺口说明 |
| --- | --- | --- | --- | --- | --- |
| AC-R08-01 | R08 | C | DONE | prd-lessons.ts:49-55；foundation_sessions.rs:382 | 不同 ID 各计一节；Rust PASS |
| AC-R08-02 | R08 | C | DONE | lessons.rs:200-213；foundation_lessons.rs:159 | 按老师 ID 汇总、课记保留旧名；Rust PASS |
| AC-R08-03 | R08 | C | DONE | prd-lesson-view.tsx:222、255-258 | 结构化 teacher_notes 可定位原课记；浏览器未跑 |
| AC-R09-01 | R09 | C | DONE | foundation_sessions.rs:382；coverage.rs:290-300 | 附录 D Rust PASS |
| AC-R09-02 | R09 | C | DONE | prd-history-view.tsx:14-28 | 700px 以下用 13 周整行按钮，无逐日小按钮 |

### 横切 A0 AC

| AC 编号 | 所属 R | 批次 | 状态 | 证据（文件:行号／测试） | 缺口说明 |
| --- | --- | --- | --- | --- | --- |
| AC-R14-01 | R14 | A0 | DONE | prd-outbox.ts:92-140；prd-outbox-contract.mjs:13-23 | operation_id 回执重放；JS／DO 合约本轮未跑 |
| AC-R14-02 | R14 | A0 | DONE | prd-outbox.ts:60-64、98-108；prd-outbox-view.tsx:27-36 | actor＋space 不符即阻断 |
| AC-R14-03 | R14 | A0 | DONE | stable-options.tsx:6-25 | 当前表单候选和顺序冻结 |
| AC-R14-04 | R14 | A0 | DONE | overlay.tsx:6-69 | Back 只关闭顶部覆盖层；浏览器未跑 |
| AC-R14-05 | R14 | A0 | DONE | prd-lesson-view.tsx:224、234；draft-store.ts:8-60 | 草稿按 ID 持久化并全局列出 |
| AC-R14-06 | R14 | A0 | DONE | http/lib.rs:408-432；api.ts:88-103；pending-request.ts | 写前 409，表单 pending／draft 保留；HTTP 合约本轮未跑 |
| AC-R15-01 | R15 | A0 | DONE | guided-field.tsx:6-10；input-guidance.tsx:4-7 | 示例只做 placeholder／独立文本 |
| AC-R15-02 | R15 | A0 | DONE | input-guidance.tsx:4-7 | 输入后仍可展开完整例子 |
| AC-R15-03 | R15 | A0 | PARTIAL | overlay.tsx:36-40、70-83 | 有 visualViewport／KeyboardAvoidingView，但本轮无 200% 文本＋真实键盘实测；历史脚本只是 zoom 模拟 |
| AC-R15-04 | R15 | A0 | DONE | readability.tsx:26-54；guided-field.tsx:10 | 尺寸源码达标；核心色值静态对比度 >4.5；浏览器测量未跑 |

## 四、数据模型与持久化现状（T44 准备）

### 4.1 迁移现状

crates/core/migrations 仍是旧模型 001–010：

| 迁移 | 表 |
| --- | --- |
| 001 | goals、sessions、session_targets、difficulties、experiences、operations |
| 002 | goal_revisions |
| 003 | auth_users、auth_sessions、auth_login_attempts |
| 004 | pathways、goal_pathways、experience_goals、experience_pathways |
| 005 | auth_registration_requests、auth_registration_limits |
| 006 | import_previews、import_batches、import_source_rows |
| 007 | lessons |
| 008 | lesson_options |
| 009 | deleted_records、session_revisions |
| 010 | lesson_targets |

证据：crates/core/src/store.rs:95-138。

当前新模型没有替换这些迁移，而是使用 crates/core/src/foundation 下独立、显式选择的 schema v1–v6：

| 版本 | 文件 | 新增表／作用 |
| --- | --- | --- |
| v1 | schema.sql | prd_schema、prd_entities、prd_operations、prd_revisions、prd_method_goals |
| v2 | lessons_v2.sql | prd_lessons、lesson revisions、knowledge、actions、lesson refs |
| v3 | sessions_v3.sql | prd_sessions、session revisions、output revisions |
| v4 | followups_v4.sql | settings、schedules、feedback、annotations、reviews 及 revisions |
| v5 | observations_v5.sql | observations、observation revisions |
| v6 | transfer_v6.sql | import previews／rows／batches、upgrade marker |

初始化会拒绝不明旧库，不把普通启动当迁移：crates/core/src/foundation/mod.rs:1-44、87-142。原生 SQLite 与 DO 都显式按 SKARMA_MODEL 选 legacy／prd_v1：crates/api/src/main.rs:228-231、crates/worker/src/runtime.rs:285-318。

判定：T44 所需“隔离合成库的新持久化基础”已大体实现；但实现形态不是 PRD §5.1 所写的“开发用 001–010 压缩后新基线 migration”，而是并行 prd_schema v1–v6。此差异需要在发布设计中明确接受，不能让 handoff 继续称 T44 完全未实施。

### 4.2 model／store 现状

- crates/core/src/model.rs 仍是 legacy 模型，只有旧 GoalStatus active／archived 和 GoalProgress，见 model.rs:11-54；它不是 PRD 0.5 新模型。
- PRD 新模型位于 crates/core/src/foundation/model.rs：
  - GoalState：not_started／active／maintaining／paused／achieved，行 4-12。
  - MethodState：active／paused／retired，行 60-66。
  - 稳定课程／老师目录 ID＋version＋active，行 113-130。
  - 实体写入带 expected_version、operation_id、change_note 和字段 presence mask，行 139-244。
- 经验／困难状态当前由 process_model::Details.state 字符串表达并在 followups.rs 校验，不是 Rust enum：process_model.rs:6-35、followups.rs:143-160。
- 所有原生调用经 IMMEDIATE transaction；DO 经 Durable Object transaction 调同一 skarma_core::execute：crates/api/src/store.rs:45-65、crates/worker/src/runtime.rs:285-318。

### 4.3 §3.4 过程记录覆盖

| PRD 过程实体 | 当前实现 | 判定 |
| --- | --- | --- |
| 复盘事件／修订 | prd_reviews、prd_review_revisions；process_model::Review | DONE |
| 复盘安排／修订 | prd_schedules、prd_schedule_revisions；每对象主键 | DONE |
| 下一步／修订 | prd_actions＋prd_output_revisions；NextAction | DONE |
| 经验使用反馈／修订 | prd_feedback、feedback revisions、annotations | DONE |
| 困难跟进 | 与 feedback 共用事件模型，按知识种类校验 | DONE |
| 待整理条目 | 嵌入 lesson 的 request_generation／organization state，而非独立表 | DONE（实现形态不同） |
| 家庭设置 | prd_settings | DONE |
| 方法版本 | prd_entities＋prd_revisions，实践保存 Method 快照 | DONE |
| 经验派生关系 | Knowledge.derived_from 嵌入并校验 | DONE |
| 计划来源 | Session.lesson_sources／action snapshots | DONE |
| 长期关注点 | Lesson／Session focus_goals 快照 | DONE |
| 瞬间观察 | prd_observations(kind=moment) | DONE |
| 目标测量 | metric／measurement observation records | DONE |
| 家人回应 | Lesson.acknowledgements，按 actor 去重 | DONE |
| 共享需要说明 | observation kind=needs，固定单条 ID＋版本 | DONE |

注意：大量结构化业务对象存为 data JSON，并由 Rust 校验，而不是每个字段都有 SQL 列／约束；数据库外写或未来兼容工具需要继续走共用层，不能假定数据库自身能完全防止非法状态。

### 4.4 契约门禁、多草稿、离线队列

- 契约门禁：DONE。写路由要求 X-Skarma-Contract=skarmar.prd.v1；旧页面写入新模型也被 core 拒绝。证据：http/lib.rs:408-572、core/lib.rs:94-140。
- 多草稿：DONE。目标／方法、课记、干预、观察、跟进分别用 account＋space＋type 键，内部再按记录 ID 分槽；支持多窗口锁和备份。证据：foundation-view.tsx:107-166、prd-session-view.tsx:45-62、prd-followup-view.tsx:37-44。
- 离线队列：PARTIAL。基础课记和目录依赖已完成；账号／空间／契约／停用选项／未知回执防护完整。缺少“未尝试上传项取消上传”，可信会话提示提前量也与附录 A 不符。
- 旧 API 字段更新：DONE。Write 记录字段 presence，省略保留、显式空／null 清除；Rust 测试 foundation.rs:587、650 本轮通过。

## 五、建议实施顺序（逐批逐切片路线图）

### A0 剩余

#### A0-1 固化新 schema 与发布边界

- 修改：crates/core/src/foundation/schema.sql、lessons_v2.sql、sessions_v3.sql、followups_v4.sql、observations_v5.sql、transfer_v6.sql、docs/runtime-storage.md、workspace-state.json。
- 目标：正式说明“并行 prd_schema v1–v6”是否取代 PRD 所述压缩 001–010；记录生产选择 SKARMA_MODEL、转换、回滚和已有 DO／账号保留策略。
- 对应：§5.1–5.3、T44。
- 测试：原生空库每级升级、旧库拒绝、显式转换回滚、DO 同级迁移；复用 foundation_transfer.rs 并恢复 test:prd-runtimes。

#### A0-2 补齐离线取消与 14 天续期

- 修改：apps/client/src/prd-outbox.ts、prd-outbox-view.tsx、auth.tsx、crates/core/src/auth.rs（如提示策略放服务端）、tools/test-prd-client.mjs、tools/smoke_prd_offline_web.py、tools/smoke_prd_auth_sessions_web.py。
- 目标：仅 queued 且 attempted=false 的本机记录／依赖可显式取消；uncertain 不允许丢；可信设备在到期前 14 天提示。
- 对应：R14、§2.2、附录 A；不改变已确认服务端记录。
- 测试：取消未发送、禁止取消 uncertain、目录依赖级联核对、账号切换、365 天会话＋14 天提示。

#### A0-3 修复 R15 偏差和可运行测试入口

- 修改：apps/client/src/foundation-view.tsx、controls.tsx、controls.web.tsx、overlay.tsx、tools/check-prd-web.mjs、package.json。
- 目标：移除常驻刷新，只在更新／失败提示中提供；原生日期控件也满足双方式；明确使用 python3；给 Node 22 的 TS 测试配置可用 runner，而不是直接 import .ts。
- 对应：R15、AC-R15-03／04。
- 测试：360／430／320、200%浏览器文本缩放、真实键盘 visualViewport、前进／后退／刷新、多层覆盖层、无横向滚动。

### A1 剩余

#### A1-1 今天页信息完整化

- 修改：apps/client/src/foundation-view.tsx、prd-lesson-view.tsx、prd-session-view.tsx、tools/smoke_prd_sessions_web.py。
- 目标：页头显示 Asia/Shanghai 业务日期；今天已记录统一展示作者、创建时间／时段和刚保存状态。
- 对应：R01。
- 测试：0／3／6 个长期方向的 360×640 首屏；课程和家庭记录时间显示；多计划选择不改版本。

#### A1-2 撤销入口与入站引用预检

- 修改：crates/core/src/foundation/lessons.rs（提供可撤销 capability／reason 读取）、crates/http/src/lib.rs、apps/client/src/prd-lesson-view.tsx、foundation_lessons.rs。
- 目标：已被引用时普通成员界面只显示标记记错，不先展示必然失败的撤销；服务端事务仍是最终权威。
- 对应：R04、AC-R04-05。
- 测试：创建者同日无引用可撤；任一引用后 capability=false；并发新增引用时提交仍被事务阻止。

### A2 剩余

#### A2-1 补目标字段

- 修改：crates/core/src/foundation/model.rs、foundation/mod.rs、apps/client/src/foundation.ts、foundation-view.tsx、field-guidance.ts、导入导出映射与测试。
- 目标：增加 domain、due 和清晰的基础帮助条件；版本化且旧记录默认值安全。
- 对应：R06、附录 B goal.domain／goal.due。
- 测试：只填名称＋类型仍通过；字段 presence patch；导出列；旧 schema v6→新版本迁移；DO 对账。

#### A2-2 总结新困难的结构化字段

- 修改：lesson_model.rs 或独立 NewKnowledgeDraft 类型、sessions.rs、prd-session-view.tsx、followups.rs。
- 目标：总结同页保存观察、成人回应、原因猜测、条件，进入 Knowledge.Details；原话／推测不混写。
- 对应：R03。
- 测试：同一总结两条困难重试；字段原子回滚；后续编辑保留来源总结版本。

### B 剩余

#### B-1 区分普通方法创建与快捷方法创建

- 修改：foundation/model.rs、foundation/mod.rs、foundation-view.tsx、prd-session-view.tsx、field-guidance.ts。
- 目标：普通方法要求具体做法；计划／总结快捷路径允许 name＋当前目标；增加独立 observe 和 scope。
- 对应：R07、附录 B method.steps／observe／scope。
- 测试：普通空做法拒绝；快捷名称-only 通过；旧实践仍指向旧版本；方法排序与心得关联不回归。

### C 收尾

#### C-1 恢复全套浏览器与双运行环境验收

- 修改：主要是测试运行器／依赖锁定；若测试暴露问题再改 prd-lesson-view.tsx、prd-history-view.tsx、coverage.rs、prd_export.rs。
- 目标：实际执行 test:prd-web、test:prd-runtimes、Web build、typecheck，并用附录 D 在 SQLite／真实 Rust Worker／DO 得到一致结果。
- 对应：R08、R09、R13 及全部跨运行环境要求。
- 测试：59 AC 对应脚本；附录 D；Excel 导出；320／360／430／宽屏；课时改名／重复节次；热力和 13 周条。

## 六、审计中发现的风险与不确定项

1. 交接状态严重滞后：T44 仍标 todo，而 3ce27f9 已实现从新 schema 到 B／C 大量功能。后续 agent 若按 handoff 从零实施，存在重复开发和破坏现有模型风险。
2. 本轮无法验证前端可构建：node_modules 缺失导致 typecheck 和 build 失败；不能把历史记录当本轮证据。
3. 双运行环境未验证：Miniflare 缺失，test:prd-runtimes 未执行。虽然 Worker 调用同一 core 且 workspace Rust 测试通过，DO SQL 行为仍需真实对账。
4. 浏览器未验证：Playwright 缺失；check-prd-web 还固定使用 python，当前解析为 Python 2。已有大量 smoke 脚本只能记为“存在”，不能记为本轮通过。
5. 新模型发布开关：wrangler.toml 当前没有 SKARMA_MODEL=prd_v1；默认仍可能走 legacy。符合“未授权生产升级”的保守边界，但不能把合成库实现称为线上已交付。
6. schema 形态与 PRD 字面不一致：旧 migrations 保持 001–010，新模型另有 prd_schema v1–v6。功能上隔离合理，但必须形成正式发布裁决。
7. SQL 层大量使用 JSON data：Rust 共用层提供强校验，数据库本身只承担 ID／版本／唯一性；任何旁路写入、外部导入和未来迁移都必须走受控命令。
8. AC 覆盖不足以代表完整 R：59 条 AC 大多已满足，但 R01 日期／时间、R03 结构化困难、R06 字段、R07 普通方法完整性、R14 队列取消与 14 天提示、R15 常驻刷新仍有 PRD 正文 gap。
9. 生产转换代码已经存在于 foundation/transfer.rs，但 PRD 的 L-2 原本要求另行裁决。代码存在不等于获得对生产执行的授权；本审计没有运行转换、部署或访问生产数据。
10. 实跑测试产生的编译／缓存产物未进入 git；审计结束工作树干净。

## BLOCKED-NEEDS

- 要完成“本轮全绿”验证，需要在本机提供与锁文件匹配的 Node 依赖、Playwright Python 包及浏览器、Miniflare，并修复或明确 Python 3／TS test runner 入口。
- 要判定可生产发布，需要用户／reviewer 明确接受并行 prd_schema v1–v6 方案、SKARMA_MODEL 切换及现有生产数据转换／回滚方案。
