# 评审修改版发布（2026-10-30）

状态：**已上线并通过公开检查**。入口：**https://skarma.vip**。

用户明确要求部署当前已推送版本。本次使用原应用普通发布流程，保留账号、会话和业务记录。

## 本次范围

应用基线 `6c8a5194a44d4c4eac079929b2d78a48cb2d0317`，包含 T58/T59/T61 的评审整改：

- 今天双入口文字居中，增加箭头及按压反馈；成对选项保持同等视觉层级，经验绿色、困难暖金，页面／卡片／按钮采用不同色阶。
- 长期关注支持共享优先级；计划先看目标与方法覆盖，再选择目标和方法、查看经验困难提示，最后填写安排。
- 总结将补充反馈和经验／困难作为可选内容，保留已有反馈及完成后修订。
- 基础记课先选课程与老师，再按需补充观察；课堂中观察到的好方法单独保留，供后续复盘使用。
- 目标详情分组、方法关联跳转、课时明细标题与复盘问题引导等整改。

`long_priority` 与 `helpful_notes` 是既有 JSON 中的可选字段；没有新 SQL schema／表、依赖或部署配置。
空值不改变旧请求序列化与身份，旧待确认回执可重放；不要求清理旧草稿或业务数据。
完整范围及截图见[评审整改记录](review-feedback-2026-10-30.md)。

## 发布前检查

- 工作区干净、远端为应用提交 `6c8a519`，普通 CI [run41065522537](https://github.com/q9good/skarmar/actions/runs/41065522537) validate success、release skipped。
- 应用摘要 `d2c820aa82eb4842d030f0e67dac1df53aede09e97e7b4051001738b0825c1f2` 与本轮 17 图复审产物一致。
- 本轮重跑 29 项发布保护、9 项演示工具保护及 120 项既有产物 SHA 校验通过；发布过程中现有 health 返回 200。
- T58 已实际通过 163 项 Rust、两套运行环境合约及重启、27 组浏览器等完整检查；T61 最后前端修改通过类型／Web 构建、62 客户端及 6 组相关浏览器流程，最终标题颜色后再复验审核流程。历史检查与本次 runner 检查分别记录。

## 实际发布结果

| 项目 | 结果 |
| --- | --- |
| 触发／部署源码 | `b668c15df9522e305e176cf8b603d4587a4a3558`（含应用 `6c8a519`） |
| Actions | [run41066365995](https://github.com/q9good/skarmar/actions/runs/41066365995)，validate／release 均 success |
| 原版本 | `266022a9-e23d-4916-bda1-52e646badb41` |
| 上传并实际激活 | `b3fdf484-0f07-4522-bc8d-8d4c5f3a84fa` |
| 模型／结构版本 | `prd_v1`／v7 |
| 回执 | [deployed_and_verified](artifacts/deployment/cloudflare-release-20261030-receipt.json) |

runner 实际执行类型检查、Web 构建、Rust 测试及 Wasm Clippy，再核对原目标、上传唯一候选、比较绑定／变量／运行配置、激活 100% 流量并验收。
本次仅使用普通发布标记；模型初始化、演示恢复及补关联步骤均 skipped。
发布运行结束前没有推进共享分支；之后才提交本记录及任务快照，不再次部署。

## 上线检查

runner 和[云端独立检查](artifacts/deployment/cloudflare-release-20261030-independent-checks.json)均通过：

- 首页仅去掉已知的一条 Cloudflare Analytics 注入后，与本次 Web 构建字节相同；JavaScript 逐字节匹配。
- health 200／status ok；匿名 me／state 401，未知 API 404。
- 无账号 Cookie 的合法 Origin logout 200；非法 Origin 或缺请求标记 403。
- 使用从未签发的随机会话检查 PRD 数据库启动成功，不登录真实账号、不读取业务数据或修改家人会话。

沿用 `skarma-web`、`SkarmaSpace/SKARMA_DB/default`、PRD v7、`skarma.vip` 和精确来源。
不重建 Worker／数据库／管理员，不自动种入内容、不恢复合成记录、不清库。
原 Cloud 环境 T12 凭据发布故障未修复；本次复用既有 GitHub Actions 安全凭据，没有提取或改写密钥。

真实账号登录、手机和完整读屏体验仍需后续验收；Opus 待办及正式 `implementationCommit` 不变。
本次发布保留原评审截图、HTML 和机器结果的测试时快照，不把其生成时的 `deployed: false` 改写成上线证据。
