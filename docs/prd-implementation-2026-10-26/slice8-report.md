# Phase 2 切片 8 实施报告（C-1：全量验收恢复与统计金标准对账）

- 基线：起始提交 7117f32；结束提交 111331e；日期 2026-10-25
- 分支：feat/tencent-informed-web
- 工作树：代码提交后干净；本地相对 origin ahead 8；未 push
- 结论：附录 D 的全部期望行已逐行进入断言；native SQLite 全量合约实跑全绿；Miniflare／DO 因本机 glibc 2.31 环境限制明确 SKIP；C 批 R08/R09/R13/专业沟通摘要已核对并补齐两处展示缺口；59 条 AC 的代码与自动化自评均为 DONE。真机与受限机器上的 DO 仍不冒充已验收。

## 一、附录 D 统计金标准对账

统一 fixture 为 tools/fixtures/prd-coverage.json。Rust 测试 crates/api/tests/foundation_sessions.rs:534-820 直接读取该文件，构造带历史目标层级、方法版本、老师稳定 ID／名称快照、状态与待核对标记的隔离 SQLite 数据。cargo test --workspace 与定向 golden test 均 PASS。

### D.1 九行窗口与目标覆盖

下表的断言输入定义在 crates/api/tests/foundation_sessions.rs:649-658，窗口起止断言在 :675-683，记录总数及 S1/S2/S3/S4/M1/M2 的整行比较在 :684-688。每一行都经过 foundation_coverage_at，未用手算结果替代实现。

| PRD 期望行 | 覆盖断言 | 本轮结果 |
| --- | --- | --- |
| 7天家庭：3／2／1／2／0／2／3 | foundation_sessions.rs:650、:665-688 | PASS |
| 7天机构：4／2／0／1／0／2／1 | foundation_sessions.rs:651、:665-688 | PASS |
| 7天合计：7／4／1／3／0／4／4 | foundation_sessions.rs:652、:665-688 | PASS |
| 30天家庭：4／2／2／2／0／3／3 | foundation_sessions.rs:653、:665-688 | PASS |
| 30天机构：5／2／1／1／0／3／1 | foundation_sessions.rs:654、:665-688 | PASS |
| 30天合计：9／4／3／3／0／6／4 | foundation_sessions.rs:655、:665-688 | PASS |
| 90天家庭：5／3／2／2／0／4／3 | foundation_sessions.rs:656、:665-688 | PASS |
| 90天机构：5／2／1／1／0／3／1 | foundation_sessions.rs:657、:665-688 | PASS |
| 90天合计：10／5／3／3／0／7／4 | foundation_sessions.rs:658、:665-688 | PASS |

边界同时被固定为 7天 2026-10-18、30天 2026-09-25、90天 2026-07-27，终点均为 2026-10-24（foundation_sessions.rs:675-683）。

### D.1 “最近一次”六行

六个目标的家庭／机构／合计三列全部由同一来源循环断言，期望定义及逐来源比较位于 foundation_sessions.rs:690-737。

| 目标 | 家庭／机构／合计期望 | 覆盖断言 | 本轮结果 |
| --- | --- | --- | --- |
| S1 | 10/24／10/24／10/24 | :691-696、:723-737 | PASS |
| S2 | 10/18／09/30／10/18 | :697-702、:723-737 | PASS |
| S3 | 10/24／10/24／10/24 | :703-708、:723-737 | PASS |
| S4 | 07/26／无记录／07/26 | :709、:723-737 | PASS |
| M1 | 10/24／10/24／10/24 | :710-715、:723-737 | PASS |
| M2 | 10/24／10/24／10/24 | :716-721、:723-737 | PASS |

S4 位于所有窗口之外但最近一次仍为 07/26，因此也验证了“最近一次不受窗口限制”。

### D.2 方法、热力图、标签、长期关注与课时

| PRD 期望 | 覆盖它的测试与断言 | 本轮结果 |
| --- | --- | --- |
| 7天：家庭 A/B/记不清=1/1/1；机构 A/B/未整理=0/3/1；合计 A/B=1/4 | foundation_sessions.rs:740-765 | PASS |
| 30天：家庭 1/2/1；机构 1/3/1；合计 2/5 | foundation_sessions.rs:740-765 | PASS |
| 90天：家庭 2/2/1；机构 1/3/1；合计 3/5 | foundation_sessions.rs:740-765 | PASS |
| 10月家庭热力图：10/18=1、10/24=2、其他为0 | foundation_sessions.rs:800-817 对完整有值日期 map 作相等比较 | PASS |
| 9月家庭热力图：09/20=1、09/30=1 | foundation_sessions.rs:810-815 | PASS |
| 近7天标签：游戏2、阅读1、未分类1 | foundation_sessions.rs:787-798 | PASS |
| 近30天标签：游戏2、阅读2、未分类1 | foundation_sessions.rs:787-798 | PASS |
| 近90天再加生活自理1 | foundation_sessions.rs:793-798 | PASS |
| LG 明确关注：家庭1／机构1／合计2 | foundation_sessions.rs:767-772 | PASS |
| LG 下级证据：7天3/3、30天4/4、90天5/4 | foundation_sessions.rs:773-785 | PASS |
| I1 瞬间观察不改变家庭完成数或热力图 | foundation_observations.rs:96-131 比较新增瞬间前后完整 coverage JSON | PASS |
| 10月 T1=2、T2=2（T2 待核对1） | tools/test-prd-client.mjs:52-62，直接读取附录 D fixture | PASS |
| 9月 T1=1且历史名“测试甲” | tools/test-prd-client.mjs:63 | PASS |
| L4 无目标仍计；L5 撤销不计；L6 标记待核对仍计 | tools/test-prd-client.mjs:54-65 | PASS |
| 老师按稳定 ID 汇总，名称快照跨月保留 | tools/test-prd-client.mjs:46-65 | PASS |
| F4 版本2只计一条、A 第2版不多算；F2 计划A实际B；S2使用当时上级 | fixture 构造 :584-647；整行覆盖 :649-688；方法断言 :740-765 | PASS |

本轮没有发现 PRD 金标准与业务实现不一致，不需要改 PRD，也没有修改期望迁就实现。原测试仅抽查了 S4 最近日期、方法 A、两个热力日、未分类和长期关注；本轮补齐了全部最近日期、方法 B、完整标签、完整热力 map、长期下级证据和两个月课时结算。

## 二、双运行环境对账

### native SQLite 半段（逐组结果）

npm run test:prd-runtimes 使用一次性 SQLite 文件与本地 skarma-api，以下 16 组全部实际运行并 PASS：

| 组 | 实跑结果 |
| --- | --- |
| 认证 CRUD／版本历史／契约门禁／重试／并发／关系／稳定名称／停用成员拒绝 | PASS |
| 课堂观察与整理分区／旧观察引用／原子产出／撤销／管理员删除／计数与待核对 | PASS |
| 计划／总结／完成／实际方法／修订产出／互斥行动／未来日期拒绝／来源保留删除 | PASS |
| 知识 CRUD／来源唯一反馈／附注／主结果修订／比较门禁／复盘／日历／行动历史 | PASS |
| 队列 actor／space／contract／目录依赖／停用选项／精确回执重放 | PASS |
| 过期 actor／space 的读写、导出、登出与重放拒绝 | PASS |
| 可信设备 TTL／公开过期元数据／分范围会话撤销 | PASS |
| 指标快照／实际值校验／来源去重／共享需要／瞬间观察单列 | PASS |
| 实际 XLSX multipart 导入／稳定来源去重／关系／原字段／精确提交重放 | PASS |
| 队列刷新返回当前名称、关系及每对象唯一安排 | PASS |
| 省略字段保留；显式空值／null 只清指定字段 | PASS |
| 方法调整切分旧证据／精确重试／有界历史／部分复盘 | PASS |
| 课记反馈原子性／旧观察精确引用／冲突／照护者状态／真实测量进展 | PASS |
| 新模型 XLSX 版本标记／数值／历史关系／无公式执行 | PASS |
| 鉴权 Excel 导出且源状态不变 | PASS |
| 服务重启后新模型、认证、不可变历史和 import-source 对账 | PASS |

退出前运行器打印：PASS native SQLite runtime: full PRD contract, restart, authentication, histories and export/import-source checks。

### Miniflare／DO 半段（环境受限的精确记录）

- 失败点：workerd 进程启动；原运行会在 Miniflare dispatch 前后表现为 workerd 崩溃与 EPIPE。
- 本机：Ubuntu 20.04，glibc 2.31。
- 当前 workerd：1.20260730.1；二进制报告缺少 GLIBC_2.32、2.33、2.34、2.35。
- 已尝试版本：当前 1.20260730.1；编排者此前也尝试 1.20250405.0，仍失败。
- Worker 产物：worker-build 0.8.7 + wasm32-unknown-unknown 已生成 crates/worker/build/index.js；阻塞不是 Worker build 产物。
- 结论：必须在 glibc ≥2.32 的机器（实际应满足该 workerd 二进制全部 GLIBC 符号，例如 Mac 或较新 Linux）上运行 DO 半段。本机没有降级 workerd，没有改 package.json／package-lock.json，也没有绕过业务逻辑。
- 本轮 npm run test:prd-runtimes 退出码 0：native 全量 PASS，DO 明确 SKIP；这不是把 SKIP 算作 PASS，也不是把环境限制算作业务失败。

### test:prd-runtimes 运行器改进

tools/check-prd-runtimes.mjs:22-30 在 native 半段之后用 workerd --version 做启动前探测；仅当 stderr 确认是 GLIBC 符号缺失时返回环境阻塞。:41 先报告 native 完整 PASS，:42-45 报告包含 workerd 版本、运行时 glibc、缺失符号和换机要求的 SKIP。

退出码语义：

- native 或业务合约失败：仍抛错并非零退出。
- workerd 非 GLIBC 原因启动失败：仍抛错并非零退出。
- 仅已识别的本机 GLIBC 环境限制：退出 0，但输出只有 SKIP Rust Worker / Durable Object，不输出 PASS both runtimes。
- glibc 可用机器：继续执行真实 Rust Worker／DO 的完整合约和重启对账。

## 三、C 批业务项验收核对

### R08（搜索／分段核对／通用 Excel 导出）

- 现状：课堂记录与课时核对分段、默认月份、单月份步进器、自定义区间折叠、稳定老师 ID 汇总、历史名称快照、待核对单列、目录设置与选择器快捷新增均已存在（apps/client/src/prd-lesson-view.tsx:225-263；apps/client/src/prd-lessons.ts:62-69）。
- 缺口：搜索此前虽然覆盖课堂文本及关联知识，但结果把课程／老师／关联困难／经验合并成泛化来源标签，未逐项标明匹配来源。
- 改动：新增 lessonSearchSources，分别返回课程、老师、老师建议、课堂观察、关联困难、关联经验；自由文本不会被标为老师建议（apps/client/src/prd-lessons.ts:48-60）。列表使用精确来源并仍可点开原课记（prd-lesson-view.tsx:229-230、:262-263、:316）。
- “只看老师建议”：仍严格检查 teacher_notes 是否实际填写，不从 notes 自由文本猜主题（prd-lesson-view.tsx:230）。
- 通用 Excel：Web 账号页保留通用导出（auth.tsx:193-198）；鉴权、版本门禁、XLSX 内容与不修改源数据由 tools/prd-export-contract.mjs:7-15、verify-prd-export.py:17 和 smoke_prd_export_web.py:28-29 覆盖；没有新增专项课时导出。
- 实测：test:prd-client 55/55；smoke_prd_lessons_web.py:72-75 通过关联困难与老师建议两种来源并回到原记录；三轮 test:prd-web 均 PASS。

### R09（热力图与时间分布）

- 现状：手机每目标整行可点并展示 13 格周条，宽度 >700px 才提供日粒度图；最近30次不限跨度且显示实际起止；月历日格 minWidth 44、minHeight 76（prd-history-view.tsx:24-28；intervention-calendar.tsx:9；history-day-plot.tsx）。
- 缺口：目标行原文为“覆盖 N 次 · 最近日期”，语义正确但没有按 PRD 明确写成“最近一次 日期（N天前）· 近90天 M次”。
- 改动：prd-history-view.tsx:26 改为 PRD 明确文案，长期方向额外括注“明确关注或下级证据”。
- 实测：smoke_prd_history_web.py:26-27 验证近90天文案、13周条和最近30次实际跨度；宽屏日点／手机周条、44px 月历及详情导航在三轮 test:prd-web 均 PASS。

### R13（色阶与进展、参与信号、瞬间不计次）

- 现状：coverage.rs:97-105 固定窗口 [今天−N+1, 今天]；:109-132 排除未完成、未来、撤销／删除；:196-200 按当前来源查询全历史最近一次；:248-259 将覆盖、直接、下级、明确关注和固定 band 分开。
- 色阶：apps/client/src/prd-coverage.ts:7 固定 7/30/90 天分档；prd-coverage-view.tsx:39 显示窗口名和图例，并明确筛选不改变颜色含义。
- 方法未知与标签：prd-coverage-view.tsx:51 分列“方法未记录”和“明确未用库中方法”，标签仅取正式家庭总结。
- 参与信号：复盘只读展示明确关联信号、日期与来源；未关联整次信号不复制到目标、不评分。专业摘要同样只带明确目标关联信号。
- 瞬间观察：foundation_observations.rs:96-131 断言新增 moment 前后完整 coverage 完全相等；moment 仍进入复盘证据。
- 缺口／改动：业务代码无新缺口；本轮主要补齐附录 D 的完整回归断言。
- 实测：Rust golden、cargo test --workspace、test:prd-client 固定色阶与图表测试、三轮 test:prd-web 均 PASS。

### 专业沟通摘要（R12/P2-16）

- 现状：入口在 prd-followup-view.tsx:159；:241 可选时间、目标、家庭／课堂／困难／瞬间／测量／共享说明，生成后可编辑删减、补问题、复制或下载，并明确不会自动发送、接收者由家人选择。
- 生成规则：prd-brief.ts:4-31 只带所选日期、目标和内容；明确区分瞬间观察、指标版本、帮助条件与家庭提供的共享需要，不输出未选目标内容。
- 缺口／改动：无新缺口。
- 实测：test-prd-client.mjs:270-281 覆盖长期方向隐私边界、日期、瞬间／测量／共享说明；smoke_prd_followups_web.py:74-76 实际生成可编辑摘要并下载；三轮浏览器测试 PASS。

## 四、全量测试实跑结果

| 命令 | 本轮真跑结果 |
| --- | --- |
| npm run typecheck | PASS |
| EXPO_NO_TELEMETRY=1 npm run web:build | PASS；Expo Web bundle 生成 |
| npm run test:planning | PASS，14/14 |
| npm run test:workflow | PASS，6/6 |
| npm run test:prd-client | PASS，55/55 |
| npm run test:prd-web（连续第1轮） | PASS，72 个 PASS 检查点 |
| npm run test:prd-web（连续第2轮） | PASS，72 个 PASS 检查点 |
| npm run test:prd-web（连续第3轮） | PASS，72 个 PASS 检查点 |
| npm run test:sync | PASS，14/14 |
| cargo test --workspace | PASS，workspace 全部单元／集成／doc tests |
| cargo fmt --all --check | PASS |
| cargo clippy --workspace --all-targets -- -D warnings | PASS |
| node tools/check-prd-maintenance.mjs | PASS |
| npm run test:prd-runtimes | native 16组 PASS；DO 明确 SKIP（glibc 环境限制）；退出0 |
| python3 -m py_compile tools/smoke_prd_*.py | PASS |

说明：在计入三轮连续全绿之前，新增的课记搜索 smoke 曾因同一详情文字出现两次触发 Playwright strict locator 歧义；该测试定位器改为对详情对话框内 first 精确匹配后，重新从零完成上述连续三轮。开发中还修正了 tools/test-prd-client.mjs 两处既有中文 mojibake 期望值；没有放宽断言。

## 五、59 条 AC 的最终状态自评

以下 DONE 表示代码及本轮自动化证据通过；不等同于真机或本机 DO 已运行，环境边界见第七节。

| AC 编号 | 状态 | 证据 | 仍缺什么 |
| --- | --- | --- | --- |
| AC-R01-01 | DONE | smoke_prd_foundation_web：360×640 下 0/3/6 长期方向首屏主按钮 | 真机另记 |
| AC-R01-02 | DONE | smoke_prd_foundation_web：显式展开长期方向、意义与最近观察 | 无代码缺口 |
| AC-R01-03 | DONE | smoke_prd_today_web／smoke_prd_sessions_web：同日待总结选择且不覆盖 | 无 |
| AC-R02-01 | DONE | smoke_prd_refinement_web：默认日期时段、目标、三次点击加一句话 | 无 |
| AC-R02-02 | DONE | test-prd-client 的关注间隔排序与 smoke 目标建议 | 无 |
| AC-R02-03 | DONE | planning/client 测试：frequency none／暂停不进入建议 | 无 |
| AC-R02-04 | DONE | smoke_prd_sessions_web：选目标显示关联经验困难 | 无 |
| AC-R03-01 | DONE | foundation_sessions + smoke_prd_sessions_web：无计划也可按实际完成 | 无 |
| AC-R03-02 | DONE | foundation_sessions：unknown 单列；附录 D 方法未知断言 | 无 |
| AC-R03-03 | DONE | session contract：标签／情境／信号均可空 | 无 |
| AC-R03-04 | DONE | completion contract／smoke：两条困难重试不重复 | 无 |
| AC-R03-05 | DONE | session tests：performed=false 不计实际覆盖 | 无 |
| AC-R03-06 | DONE | 7117f32 + session UI/contracts：总结内只填名称快速建方法并关联实际目标 | 无 |
| AC-R04-01 | DONE | smoke_prd_lessons_web：默认今天、常用项、四次内提交 | 无 |
| AC-R04-02 | DONE | lesson client/浏览器测试：选课程不预选老师 | 无 |
| AC-R04-03 | DONE | smoke_prd_lessons_web：补观察不增节数 | 无 |
| AC-R04-04 | DONE | withdrawal capability + 浏览器：创建当天无引用可撤销、节数减少 | 无 |
| AC-R04-05 | DONE | capability tests：非创建者／隔日／引用后仅标记记错 | 无 |
| AC-R05-01 | DONE | prd-lesson-view ObservationFields：自由文本在四主题前；浏览器覆盖 | 无 |
| AC-R05-02 | DONE | smoke_prd_lessons_web：主题折叠再展开保留文本 | 无 |
| AC-R05-03 | DONE | foundation_lessons + smoke：观察／整理分区独立版本并发成功 | 无 |
| AC-R05-04 | DONE | smoke_prd_lessons_web：同区冲突保留输入且禁止覆盖 | 无 |
| AC-R05-05 | DONE | smoke：老师建议未选任务时 actions 为空 | 无 |
| AC-R06-01 | DONE | goal contracts／浏览器：名称+层级可保存 | 无 |
| AC-R06-02 | DONE | smoke_prd_refinement_web：切模板不覆盖已写文字 | 无 |
| AC-R06-03 | DONE | progress/client tests：分母0与做到数超界不显示伪成功率 | 无 |
| AC-R07-01 | DONE | method detail smoke：就地心得明确关联方法 | 无 |
| AC-R07-02 | DONE | method version tests：旧实践保留版本1 | 无 |
| AC-R07-03 | DONE | test-prd-client prioritizeMethods：仅达成目标的方法排后 | 无 |
| AC-R08-01 | DONE | smoke_prd_lessons_web:97-104：同日同课同老师不同ID计两节 | 无 |
| AC-R08-02 | DONE | test-prd-client:46-65：稳定老师ID跨月汇总并保留旧名 | 无 |
| AC-R08-03 | DONE | lessonSearchSources + smoke_prd_lessons_web:72-75：老师建议／关联知识来源标签并打开原课记 | 无 |
| AC-R09-01 | DONE | foundation_sessions.rs:800-817：10/18=1、10/24=2、完整 map 无其他10月日 | 无 |
| AC-R09-02 | DONE | prd-history-view + smoke_prd_history_web：360px整行13周条，无窄日按钮 | 真机另记 |
| AC-R10-01 | DONE | followup contracts/smoke：另一家人反向附注可见，总数仍1 | 无 |
| AC-R10-02 | DONE | followup tests：主结果版本修订移动分类，总量不增 | 无 |
| AC-R10-03 | DONE | smoke_prd_followups_web：可直接选已有帮助阶段 | 无 |
| AC-R11-01 | DONE | followup tests：两条可比观察不提示三条门槛 | 无 |
| AC-R11-02 | DONE | followup smoke：三条且无变化只建议每周回看，不增训练量 | 无 |
| AC-R12-01 | DONE | review calendar tests：10/24到期、11/14补做后为11/21 | 无 |
| AC-R12-02 | DONE | review-queue client/smoke：同方法挂两目标只一项 | 无 |
| AC-R12-03 | DONE | review/action/plan smoke：下一步可纳入目标计划 | 无 |
| AC-R12-04 | DONE | review concurrency contract：两成员并发只生成一个下期 | 无 |
| AC-R12-05 | DONE | review evidence tests：只列明确关联两条信号，整次未关联不列 | 无 |
| AC-R13-01 | DONE | 本轮附录 D 九行+六行最近日期+全部 D.2 断言 | DO 换机复跑 |
| AC-R13-02 | DONE | foundation_observations.rs:96-131：新增 moment 前后 coverage 完全相等 | 无 |
| AC-R13-03 | DONE | golden 与 planning tests：中期直接+子目标同来源去重 | 无 |
| AC-R14-01 | DONE | smoke_prd_offline_web：同请求重放仅一节正式课记 | 无 |
| AC-R14-02 | DONE | outbox client/browser：切账号不以新账号上传旧队列 | 无 |
| AC-R14-03 | DONE | multi-window/refinement smoke：编辑时新状态不改输入与排序 | 无 |
| AC-R14-04 | DONE | overlay smoke：浏览器 Back 只关闭覆盖层并保留表单 | 无 |
| AC-R14-05 | DONE | draft windows/today smoke：跨日课记草稿全局恢复 | 真机持久化另记 |
| AC-R14-06 | DONE | runtime/outbox contracts：旧契约写前拒绝，原输入保留 | 无 |
| AC-R15-01 | DONE | smoke_prd_r15_web：空白示例不进请求／导出 | 无 |
| AC-R15-02 | DONE | smoke_prd_r15_web：已有输入时问句与例子可读 | 无 |
| AC-R15-03 | DONE | smoke_prd_r15_web：200%缩放、缩小 visual viewport、焦点不被栏遮挡 | 真键盘真机另记 |
| AC-R15-04 | DONE | smoke_prd_r15_web：字号、44px点击区、4.5:1对比自动测量 | 真机另记 |
| AC-R16-01 | DONE | smoke_prd_polish_web／model smoke：页头、登录、metadata、图标均为 skarmar | 无 |
| AC-R16-02 | DONE | release/model tests：显示名变化不改 Worker／DO／域名／空间标识 | 未访问生产 |

汇总：DONE 59，PARTIAL 0，MISSING 0。限制是运行环境与真机证据，不是把未实现代码标成 DONE。

## 六、本切片发现的新缺陷（未修）

无。

本轮发现并已修：

1. R08 搜索匹配来源标签过于泛化，现已按课程／老师／老师建议／课堂观察／关联困难／关联经验精确标注。
2. R09 目标行文案未按 PRD 显式写“最近一次…·近90天…”，现已对齐。
3. 附录 D 测试只抽查部分期望，现已补齐逐行断言。
4. test:prd-runtimes 的 glibc 不兼容原本暴露为 EPIPE 堆栈，现已转为明确环境 SKIP。
5. 两处既有测试期望中文 mojibake 已纠正；未放宽断言。

## 七、未做与原因

- 未在 Miniflare／真实 Rust Worker DO 上完成本机对账：glibc 2.31 无法启动 workerd；需换 glibc ≥2.32 且满足当前二进制符号的机器。
- 未做移动浏览器实体设备与真机键盘操作；本轮只做 Playwright 320/360/430px、200%缩放、visual viewport、点击区和对比度自动验收，不冒充真机。
- 未访问生产、未部署、未改 wrangler.toml、未设置生产 SKARMA_MODEL、未访问账号或业务数据。
- 未实现 L-2 生产存储转换；PRD 未决且本切片禁止。
- 未实现 L-3 新模型 Excel 导入映射裁决；保留既有能力与测试，但未把未决映射当成已定方案。
- 未新增专项课时核对导出；按 L-4 只保留通用 Excel 业务导出。
- 未修改 miniflare／wrangler／workerd 依赖版本。

## BLOCKED-NEEDS

1. 在 Mac 或较新 Linux（glibc ≥2.32，且能启动当前 workerd 1.20260730.1）运行 npm run test:prd-runtimes，取得 Rust Worker／Durable Object 半段真实 PASS 记录。
2. 在目标移动浏览器／真机补做触控、软键盘、系统返回、200%文字缩放与离线恢复人工验收。
