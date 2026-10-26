# 对抗验收轮：审计报告 + 切片 1–4（合并复核）

## 你的角色

你是**对抗性验收方**（reviewer / opus）。worker（gpt）已完成一份实现符合性审计和四个
实现切片。你的任务不是读一遍点头，而是**独立重验**：把它声称已实现/已测试的条目抽查到
源码，并**自己重跑测试**，抓出虚报、漏报、误判，以及「为了让测试变绿而放宽断言」的行为。


这些结论将决定代码是否推送，所以基线必须站得住。
## 必读输入（用你的本地文件工具读取，路径在 Herdr 主机上）

1. `/home/wangzhichao.wzc/workspace/AI/skarmar/agent_workspace/20261024/prd-review/impl/phase1-audit.md`
   — worker 的 PRD 0.5 实现符合性审计（534 行，逐 R / 逐 59 条 AC 判定）
2. `/home/wangzhichao.wzc/workspace/AI/skarmar/agent_workspace/20261024/prd-review/impl/slice1-report.md`
   — 切片 1 报告（A0-3：测试入口修复 + R15 对齐），提交 `4d65479`
3. `/home/wangzhichao.wzc/workspace/AI/skarmar/agent_workspace/20261024/prd-review/impl/slice2-report.md`
   — 切片 2 报告（覆盖层 Back 竞态 + A0-2 离线取消/会话续期 + 测试基础设施），提交 `ec03810`
4. `/home/wangzhichao.wzc/workspace/AI/skarmar/agent_workspace/20261024/prd-review/impl/slice3-report.md`
   — 切片 3 报告（A1-1 今天页业务日期与已记录信息），提交 `f1ecb35`
5. `/home/wangzhichao.wzc/workspace/AI/skarmar/agent_workspace/20261024/prd-review/impl/slice4-report.md`
   — 切片 4 报告（A1-2 课记撤销入口与入站引用预检，AC-R04-05），提交 `c8a581e`
6. `/home/wangzhichao.wzc/workspace/AI/skarmar/agent_workspace/20261024/prd-review/impl/slice5-report.md`
   — 切片 5 报告（A2-1 目标 domain／due／基础帮助条件）；提交号见报告头部与 `git log`
7. `/home/wangzhichao.wzc/workspace/AI/skarmar/agent_workspace/20261024/prd-review/impl/slice6-report.md`
   — 切片 6 报告（A2-2 总结新建困难的结构化字段），提交 `b3a6f71`
8. `/home/wangzhichao.wzc/workspace/AI/skarmar/agent_workspace/20261024/prd-review/impl/slice7-report.md`
   — 切片 7 报告（B-1 普通方法创建与快捷方法创建区分），提交 `7117f32`
9. `/home/wangzhichao.wzc/workspace/AI/skarmar/agent_workspace/20261024/prd-review/impl/slice8-report.md`
   — 切片 8 报告（C-1 全量验收恢复与统计金标准对账）；提交号见报告头部与 `git log`
10. `/home/wangzhichao.wzc/workspace/AI/skarmar/docs/prd-family-intervention.md` — PRD 0.5（判定基准，附录 C 59 条 AC）
11. 仓库源码：`/home/wangzhichao.wzc/workspace/AI/skarmar`（验收基线为**当前 HEAD**，
    即切片 8 的提交；先 `git log --oneline -12` 与 `git status` 确认工作树干净、HEAD 是切片 8）

## 环境状态（重要，与 worker 写审计时不同）

worker 写 phase1-audit 时 node_modules／Playwright／Miniflare 缺失，把 typecheck／
web:build／浏览器测试判为「FAIL（环境）」。编排者随后已装齐：
- `npm ci`（561 包，tsc／miniflare 就绪）
- `pip3 install playwright` + `python3 -m playwright install chromium`
- `pip3 install --user openpyxl`
- `rustup target add wasm32-unknown-unknown` + `cargo install worker-build --version 0.8.7`（已装好）

所以：**审计里的「环境 FAIL」不代表代码缺陷**，以你重跑的真实结果为准。你现在可以且
应当实跑全部测试，包括浏览器套件。

编排者已独立复跑并确认（供你交叉核对，但你必须自己重跑）：
- `npm run typecheck` exit 0；`test:planning` 14/14；`test:workflow` 6/6；
  `test:prd-client` 49/49（切片 1 时）；`test:sync` 14/14；`cargo test --workspace` 148 passed
- 切片 2 后：`smoke_prd_overlay_web.py` 3/3 PASS、`smoke_prd_observations_web.py` 3/3 PASS
- 切片 2 前（基线 `4d65479`）编排者曾复现 `smoke_prd_observations_web.py` 失败
  （`Locator.click: Timeout 30000ms exceeded` 等待「删除瞬间观察」按钮），隔离连跑 3 次为
  PASS/PASS/FAIL，约 1/3 失败率——即切片 2 修的竞态是真实存在的间歇缺陷。
- **编排者已查明的环境限制（2026-10-25，勿重复排查）**：`test:prd-runtimes` 的
  **Miniflare／Durable Object 半段在本机无法运行**。根因：workerd 需要 glibc ≥2.32，
  本机是 Ubuntu 20.04 / glibc 2.31；直接运行 workerd 报 `GLIBC_2.32/2.33/2.34/2.35
  not found`，换旧版 workerd（1.20250405.0）同样失败——是系统 glibc 版本问题，与
  workerd 版本、worker build 产物无关。编排者已生成 `crates/worker/build/index.js`
  （worker-build 0.8.7，gitignored），native SQLite 半段全部 PASS，但 DO 半段在本机
  必然 EPIPE／workerd 崩溃。handoff 里 T36 的 Miniflare DO 通过记录来自 **Mac**，不是本机。
  因此你**不需要也无法**在本机跑通 DO 半段：如实记为「环境受限（glibc 2.31 < 2.32），
  本机无法验证，历史在 Mac 验证过」，**不要**为跑通而降级 workerd／改 package-lock
  （仓库级依赖变更，越界）；native SQLite 半段你应当实跑并核对（不依赖 workerd）。
- 编排者在 HEAD `c8a581e` 上已独立复跑全绿：typecheck、web:build、planning 14、
  workflow 6、prd-client 53、sync 14、cargo 149 passed/0 failed、cargo fmt、
  check-prd-maintenance、test:prd-web 连续 3/3 PASS。你的重跑应与此一致。

## 复核任务

### A. 重跑全量测试（必做，这是本轮的核心证据）

自己实际运行并记录真实结果：
```
npm run typecheck
EXPO_NO_TELEMETRY=1 npm run web:build
npm run test:planning
npm run test:workflow
npm run test:prd-client
npm run test:sync
cargo test --workspace
npm run test:prd-web          # 完整浏览器套件，至少连跑 3 次
node tools/check-prd-maintenance.mjs
```
另外**尝试** `npm run test:prd-runtimes`：worker-build 0.8.7 与 wasm32 target 现已安装，
但 `crates/worker/build/index.js` 需要先生成（`(cd crates/worker && worker-build --release)`，
见 `docs/runtime-storage.md:109-111`）。你可以生成该产物（它是 gitignored 的构建产物，
不是源码改动）后重跑，从而验证真实 Rust Worker／DO 合约——这是 worker 两轮都没能做到的。
若仍失败，精确记录失败点与原因。

核对：
- 结果是否与五份切片报告一致（数量、通过/失败）；
- worker 有没有把「跳过」「编译通过」说成「测试通过」；
- 浏览器套件是否**稳定**绿（连跑 3 次，记录每次结果；若出现失败，隔离复现并量化失败率）；
- 有没有报告未提、但实际会失败的检查。

### B. 抽查审计的 DONE 判定（防虚报）

从 phase1-audit 判为 `DONE` 的条目里**按风险抽样至少 12 条**（优先抽 A0/A1 当前批次的、
以及 worker 标注「无自动化测试」或「浏览器未跑」的 DONE），逐条打开它给的文件:行号核对：
- 该处代码是否真的实现了 PRD 对应 R/AC 的要求（不是空壳/TODO/占位/旧模型残留）；
- 行号引用是否准确（允许小幅偏移，内容必须对得上）；
- 有没有把「相关但不等于」的代码当成已实现（例如把 legacy `crates/core/src/model.rs`
  的旧模型当成 PRD §3.3 新状态模型的实现）；
- 特别注意 worker 自己在审计第六节指出的风险：新模型是并行 `prd_schema v1–v6` 而非
  PRD §5.1 字面的「压缩 001–010」，且 `wrangler.toml` 无 `SKARMA_MODEL=prd_v1`（默认走
  legacy）。核对这个判断是否准确，以及它对各 AC 的 DONE 判定意味着什么
  （在 legacy 默认下，这些 AC 算「已实现」还是「已实现但未启用」？）。

### C. 抽查 MISSING/PARTIAL/DEVIATION（防漏报与低估）

抽 5–8 条，独立在源码里搜一遍，确认判定准确、严重性未被低估。

### D. 核对 AC 覆盖完整性

PRD 附录 C 有 **59 条 AC**（第三轮定案稿确认：57 条现有 + 作者补入 AC-R03-06、
AC-R12-05）。核对审计的逐 AC 表：
- 是否 59 条都判了（无遗漏编号）；
- 批次归属（A0/A1/A2/B/C）是否与 PRD §7 一致；
- 有没有把后续批次的 AC 误判为当前批次应交付，或反之。

### E. 验收切片 1（A0-3）

逐条核对：
- 测试入口修复是否真的修好（`--experimental-strip-types` 是否沿用仓库既有 `node --test`
  约定而非引入新工具；`python3`/`PYTHON3` 改法；19 个 smoke 脚本的 Chromium 路径可配置）；
- R15 改动是否符合 PRD 第 417 行「不常驻手动刷新主按钮；有更新／失败时仍可提供刷新和重试」；
- AC-R15-01…04 的实测方式是否可信：worker 声称用浏览器读 **computed style** 与
  **实际对比度**（不是源码常量推定），并用 CDP `Emulation.setPageScaleFactor=2` 模拟 200%
  缩放、用 visualViewport 缩高模拟软键盘。打开 `tools/smoke_prd_r15_web.py` 核对这些断言
  是否真的存在且真的在测 PRD 要求的数值（≥16px 输入框、≥17px 标签与主按钮文字、
  ≥14px 说明、≥44px 热区、≥4.5:1 对比度、无横向滚动）；
- worker 是否诚实标注了「桌面模拟非真机」（PRD §7 与 T43 都要求不冒充真机验收）。

### F. 验收切片 2（A0 可靠性 + A0-2 + 测试基础设施）

逐条核对：
1. **覆盖层 Back 竞态**：`apps/client/src/overlay.tsx` 的 `useLayoutEffect` + `historyReady`
   门控是否真能消除「可见对话框无对应 history entry」的状态；`cancelled` 是否防住迟到的
   pushState；新增的 `tools/smoke_prd_overlay_web.py` 是否是**确定性**测试（通过截住/放行
   真实 `history.back` 构造 pending，而不是靠 sleep/重试）；是否覆盖 R14 第 394 行要求的
   前进/返回、刷新重载、多次开关不积累哨兵，以及 AC-R14-04。
2. **离线队列取消**：`prd-outbox.ts` 的 `cancel` 是否严格只允许 `queued && attempted===false`；
   `uncertain` 不可丢弃是否有**多层**保证（服务层校验 + UI 不给入口 + 测试比对失败前后
   raw 不变）；目录依赖级联是否正确（共享目录不误删、孤儿目录才删）；跨账号阻断是否守住。
   核对 `tools/test-prd-client.mjs` 与 `tools/smoke_prd_offline_web.py` 的新增断言是否真的
   在测这些，而不是空断言。
3. **可信会话 14 天续期提示**：架构选择（服务端 `auth.rs` 还是客户端 `auth.tsx`/
   `session-renewal.ts`）是否合理并说明理由；提示是否为站内非阻断（不是模态框，与
   R01/P1-10 一致）；365 天与提前 14 天是否与附录 A 参数一致；重新认证后是否回到原表单。
4. **测试基础设施**：`tools/browser-test-requirements.txt` 是否只含测试依赖且**未**并入
   `tools/tencent-requirements.txt`；`prd-transfer-contract.mjs`/`prd-export-contract.mjs`
   的 python3 统一是否到位。
5. **有没有为了让测试变绿而放宽断言/删测试/加 sleep**：对比 `git show ec03810` 与
   `git show 4d65479` 的 diff，检查既有断言是否被削弱。这是对抗验收的重点。

### G. 验收切片 3（A1-1 今天页业务日期与已记录信息）

逐条核对（提交 `f1ecb35`，新增 `apps/client/src/prd-today.ts`、`prd-today-view.tsx`、
`tools/smoke_prd_today_web.py`）：
1. **业务日期时区**：`prd-today.ts` 的 `businessTime` 用 `Intl.DateTimeFormat` 且
   `timeZone:'Asia/Shanghai'`；`businessDateLabel` 的星期用 `Date.UTC` 计算（不受浏览器
   本地时区影响）。核对是否真的按 Asia/Shanghai 而非本地时区，跨时区访问是否稳定。
2. **今天已记录**：`todayRecords` 是否覆盖课记（status=recorded）、家庭干预（排除
   deleted/cancelled）、瞬间观察（kind=moment 且未删除）三类；每条是否带作者
   （creator_name）、时间/时段、同步状态（「刚保存」窗口逻辑）；排序是否按 created_at
   倒序且稳定。
3. **首屏不回归**：AC-R01-01（0/3/6 个长期方向首屏可见两个主按钮）是否仍通过；新增的
   日期与已记录信息是否把主按钮挤出 360×640 首屏；布局优先级是否符合 PRD R01
   （长期方向一行 → 两个主按钮 → 今天已记录 → 待办卡）。
4. **多浏览者**：两账号共享刷新后作者显示是否正确（`smoke_prd_today_web.py` 的
   cross-account author refresh 断言是否真的在测这个）。
5. **worker 自述的中途修正**：报告称统一卡最初用通用 accessible label 导致 lesson smoke
   失败，改为在 `prd-today-view.tsx` 恢复既有课记/家庭详情的可访问入口语义。核对这个
   修正是**恢复了既有语义**（正确），而不是**改测试断言迁就实现**（错误）。
6. **断言是否被削弱**：对比 `git show f1ecb35`，检查既有测试断言有无被放宽。

### H. 验收切片 4（A1-2 课记撤销入口与入站引用预检，AC-R04-05）

逐条核对（提交 `c8a581e`，`crates/core/src/foundation/lessons.rs` +135、
`crates/api/tests/foundation_lessons.rs` +122、客户端门控、HTTP 合约）：
1. **capability 判定与事务权威一致**：`lessons.rs` 的 capability 是否严格「创建者 +
   创建当日（服务器业务日）+ 无入站引用」；客户端预检只是 UX，最终写操作是否在服务端
   事务里重新计算（并发新增引用时仍拒绝）；`reason_code` 是否覆盖 allowed/not_creator/
   referenced/outside_creation_day。
2. **入站引用定义**：哪些算入站引用（知识、下一步、计划、复盘、测量等）是否与
   `prd_lesson_refs`/事务拒绝逻辑一致；有没有遗漏的引用类型。
3. **客户端按 capability 渲染**：可撤销只显示撤销、不可撤销只显示「标记记错」；
   是否真的不再显示必然失败的撤销入口；fail-safe（未知/读取失败绝不显示撤销）。
4. **标记记错路径**：标记后在课时明细（R08）醒目显示；管理员删除仍可用；
   待核对仍计节数（PRD 要求，不得自动剔除）。
5. **AC-R04-05 三场景**：同日无引用可撤 / 引用后 capability=false 只给记错 /
   并发新增引用事务仍阻止——核对 `foundation_lessons.rs` 与
   `smoke_prd_lessons_web.py` 的断言是否真的在测这三个场景，而非空断言。
6. **断言是否被削弱**：对比 `git show c8a581e`，检查既有撤销/删除权限断言有无被放宽。

### I. 验收切片 5（A2-1 目标 domain／due／基础帮助条件）

逐条核对（提交号见 slice5-report 头部与 `git log`）：
1. **domain 单一领域控件**：是否按附录 B 实现；是否**单一领域**（PRD 明确不是多选）；
   是否可选（不填可保存）；选项集合是否走稳定目录（ID+version+active）而非前端硬编码。
2. **due 预期时间**：是否复用切片 1 的 DateField（日历/数字双方式）而非新造控件；
   是否可选；文案是否制造压力（PRD 立场：due 是家庭自填预期，不是系统强加截止日）。
3. **基础帮助条件**：R06「情境—期待行为—允许帮助—判断标准」中「允许帮助」是否有清晰
   承载；是否**未**做成必填五问（PRD 明确适度挑战说明放「看例子」）。
4. **版本化与旧记录默认值安全**：新字段是否走 `expected_version`/`operation_id`/
   presence mask；旧记录（无新字段）读取是否默认值安全、不报错、不把未填当已清空。
5. **导入导出边界**：导出侧是否补了新字段列；是否**未**实现导入侧新映射（L-3 未决）。
6. **前端与 R15 三层引导**：「更多设置」折叠是否承载 domain/due 不挤占主路径；
   示例是否不进入 value/请求/统计/导出。
7. **断言是否被削弱**：对比切片 5 的 diff，检查既有目标相关断言有无被放宽。

### J. 验收切片 6（A2-2 总结新建困难的结构化字段）

逐条核对（提交 `b3a6f71`）：
1. **结构化字段**：总结同页新建困难是否保存观察、**成人回应**、**原因猜测**、**条件**；
   字段名/问句/placeholder/示例是否按附录 B 原文（不是自创）。
2. **原话／推测分离**：数据结构上是否分开（`knowledge_details` 的不同字段），展示是否
   不混写，推测是否可被识别为推测（PRD「原因写成假设」）。
3. **旧客户端语义**：`knowledge_details: Option<BTreeMap<...>>` 的 `None` 是否表示
   「旧客户端未提交结构化详情」，而 present 条目可显式设置或清除——核对这个语义没有
   把「未填」当成「已清空」。
4. **0–N 条原子**：一次总结新建多条困难/经验是否同事务原子（复用既有 AC-R03-04 的
   operation_id 幂等），有无破坏。
5. **不重复实现参与信号**：D-2 裁决的「一组参与信号 chips」是前序切片已实现的，
   本切片不应重复新增。
6. **断言是否被削弱**：对比 `git show b3a6f71`。

### K. 验收切片 7（B-1 普通方法创建与快捷方法创建区分）

逐条核对（提交 `7117f32`）：
1. **普通方法创建要求 steps**：方法页完整新建是否要求具体做法（`method.steps`），
   空做法是否被**服务端**拒绝（不只是前端提示）。
2. **快捷创建只要 name + 目标关联**：计划 R02／总结 R03 的快捷路径是否允许只填名称并
   继承当前明确目标关联。
3. **两条路径服务端明确区分**：是否用显式创建模式/来源标记实现，而**不是**靠前端绕过
   校验；普通路径不因快捷放宽、快捷路径不因普通失败。
4. **方法版本与排序不回归**：`prd_entities`/`prd_revisions` 版本、实践保存方法快照
   （AC-R07-02）、五级排序含 D02「已完成目标关联方法靠后」与 rank 4 在普通 active 后
   （AC-R07-03）是否未回归。
5. **断言是否被削弱**：对比 `git show 7117f32`（该提交改了 30 个文件、多个 smoke 脚本，
   重点核对 smoke 脚本的断言是被**适配新行为**还是被**放宽**）。

### L. 验收切片 8（C-1 全量验收恢复与统计金标准对账）

逐条核对（提交号见 slice8-report 与 `git log`）：
1. **附录 D 金标准逐行覆盖**：worker 是否给出「附录 D 每行期望值 → 覆盖它的测试
   文件:行号与断言」的映射；抽查其中至少 5 行，确认断言真的在测那一行，且期望值与
   PRD 附录 D 一致（不一致时应以 PRD 为准修代码，不是改期望值）。
2. **native SQLite 半段**：是否完整实跑并全绿，逐组结果是否记录。
3. **DO 半段的处理是否诚实**：是否如实记为「环境受限（glibc 2.31 < 2.32），本机无法
   验证」而**没有**降级 workerd、改 `package.json`/`package-lock.json` 版本、或改业务
   逻辑绕过；`test:prd-runtimes` 的运行器改进是否输出清晰的 SKIP 而非 EPIPE 堆栈，
   且退出码语义诚实（不把 SKIP 报成 PASS，不把环境受限报成业务失败）。
4. **C 批业务项**：R08（搜索/分段核对/通用 Excel 导出，且**未**另设专项课时导出——
   L-4 裁决）、R09（13 格周条、>700px 才日粒度、日格 ≥44px、最近 30 次标起止）、
   R13（窗口含两端、终点为实际今天、每窗口固定色阶、「记不清」单列、参与信号未选＝
   未记录、瞬间不计次）、专业沟通摘要（R12/P2-16，家人自选接收者、不自动发消息、
   不建教师账号）逐项核对。
5. **59 条 AC 最终自评是否诚实**：worker 报告第五节给出逐 AC 自评，抽查其中判为 DONE
   的至少 8 条，独立核对源码；虚报要抓出来。
6. **断言是否被削弱**：对比切片 8 的 diff。

### M. 范围纪律核对

八个切片是否越界：
- 有没有动 L-2（生产存储转换）／L-3（Excel **导入**映射）这两个 PRD 未决项；
- 有没有改 `wrangler.toml` 生产绑定、设 `SKARMA_MODEL` 生产开关、部署、访问生产数据；
- 有没有改 PRD 文档本身（本轮是实现，不应改 PRD）；
- 有没有改依赖版本（`package.json`/`package-lock.json` 的 miniflare/wrangler/workerd）；
- 提交是否只在本地（未 push）。

## 判定

- `VERDICT: PASS` — 审计与八个切片可信，测试真实通过且稳定，无越界，可作为推送与后续
  切片的基线（允许少量小瑕疵，单列不阻断）。
- `VERDICT: REVISE` — 存在实质问题（虚报 DONE、测试不稳定或结果不符、断言被削弱、
  越界改动、AC 覆盖有误），编号列出必须修复项。

## 约束

- **只读复核**：不修改任何源码、测试、PRD、配置；不提交、不推送。
  例外：可以生成 gitignored 的构建产物（`crates/worker/build/`）以运行 test:prd-runtimes，
  可以安装/使用已有依赖，可以运行测试产生临时数据库。
- **独立重验优先**：worker 声称的，能自己验证就自己验证；不能验证的（真机、生产）
  明确标注「无法独立复核」，不默认采信。
- 抓错要给证据（文件:行号、测试输出、命令与退出码），不要只说「可疑」。
- 区分**阻断性问题**与**小瑕疵**，后者单列且不导致 REVISE。
- 不要重写 worker 的报告；你是判定它可不可信、哪里不可信。

## 输出格式（写入 out 文件）

```markdown
# 对抗验收报告：审计 + 切片 1–8

- 复核基线：HEAD 提交；日期；复核方式（重跑测试 + 源码抽查 + diff 审查）

## 结论
VERDICT: PASS 或 VERDICT: REVISE
（总判断 + 阻断性问题数量）

## 一、我重跑的测试（真实结果）
表格：命令 | 我的结果 | worker 报告的结果 | 是否一致 | 关键输出/退出码
浏览器套件连跑次数与每次结果；test:prd-runtimes 的尝试结果（含是否生成了 worker build）

## 二、审计 DONE 抽查（≥12 条）
表格：AC/R 编号 | worker 判定 | 我核对的文件:行号 | 我的结论（确认/虚报/偏差）| 说明
含对「并行 prd_schema v1–v6 + 默认 legacy」这一风险点的独立判断

## 三、MISSING/PARTIAL/DEVIATION 抽查（5–8 条）

## 四、AC 覆盖完整性（59 条）

## 五、切片 1 验收（A0-3）
测试入口 / R15 / AC-R15-01…04 实测方式可信度 / 真机边界诚实性

## 六、切片 2 验收（A0 可靠性 + A0-2 + 测试基础设施）
竞态修复 / 离线取消 / 会话续期 / 测试基础设施 / **断言是否被削弱**

## 七、切片 3 验收（A1-1 今天页）
业务日期时区 / 今天已记录三类与作者时间状态 / 首屏不回归 AC-R01-01 / 多浏览者作者 /
**accessible label 修正是恢复既有语义还是改测试迁就实现** / 断言是否被削弱

## 八、切片 4 验收（A1-2 课记撤销 capability）
capability 与事务权威一致 / 入站引用定义完整 / 客户端门控 / 标记记错醒目 /
AC-R04-05 三场景断言真实 / 断言是否被削弱

## 九、切片 5 验收（A2-1 目标 domain/due/帮助条件）
domain 单一领域/可选/稳定目录 / due 复用 DateField 不制造压力 / 允许帮助非必填五问 /
版本化与旧记录安全 / 导出补列未动导入 / R15 三层引导 / 断言是否被削弱

## 十、切片 6 验收（A2-2 总结新建困难结构化字段）
观察/成人回应/原因猜测/条件 / 原话推测分离 / knowledge_details 旧客户端语义 /
0–N 原子 / 不重复实现参与信号 / 断言是否被削弱

## 十一、切片 7 验收（B-1 普通 vs 快捷方法创建）
普通要求 steps 服务端拒绝 / 快捷只要 name+目标关联 / 服务端显式区分 /
方法版本与排序不回归 / smoke 脚本断言是适配还是放宽

## 十二、切片 8 验收（C-1 全量验收与统计金标准对账）
附录 D 逐行映射抽查 / native SQLite 全绿 / DO 半段处理诚实（未降 workerd/未改依赖）/
C 批业务项（R08/R09/R13/专业沟通摘要）/ 59 条 AC 自评抽查 / 断言是否被削弱

## 十三、范围纪律核对（越界检查，八个切片）

## 十四、阻断性问题（若 REVISE，编号列出修复要求）

## 十五、小瑕疵（不阻断）

## BLOCKED-NEEDS（如有）
```

## Output contract

Write the complete result to the out file path given in the dispatch instruction, using your local file tools (the path is on the Herdr box, not in your sandbox), then create the same path with `.done` appended containing exactly `DONE`. Without the marker the round is treated as unfinished.
