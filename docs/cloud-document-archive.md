# Set up skarmar 云端方案文档索引

2026-10-19 核对原云端任务「Set up skarmar」最后一次交接，主要方案与任务清单已提交到
`feat/tencent-informed-web` 的 `f0ea4430f601fcea1283da99c1e549af4ada386a`。
该提交已在本地 Git 历史中。本次从该提交逐文件导出到仓库外的文档归档，保留原样，
并校验各文件 SHA-256 和 ZIP 完整性。归档不覆盖本地代码或当前文档。

## 重点文档

| 用途 | 当前维护版本 |
| --- | --- |
| 整体产品规划、分期范围及后续语音／游戏／App | [产品规划](product-plan.md) |
| 完成状态、后续待办及阻塞 | [任务清单](workspace-state.json)、[开发交接](handoff.md) |
| 目的、途径、经验和原则设计 | [业务与技术模型](purpose-pathway-experience.md) |
| 环形图和训练覆盖统计定义 | [规划总览](planning-overview.md) |
| Rust、SQLite、Worker 和 Durable Object | [运行与存储设计](runtime-storage.md) |
| 开发和验证 | [开发说明](development.md) |
| 腾讯文档结构核对与迁移边界 | [腾讯文档核对](tencent-audit.md) |
| Excel 格式 | [导出方案](excel-export.md) |
| 当前完整 Cloudflare 托管 | [部署维护手册](cloudflare-hosted-deployment.md) |
| 大陆访问候选 | [访问评估](mainland-access-plan.md)、[默认 HTTPS 平台](default-https-platforms.md) |

云端原版本可在 [Git 历史快照](https://github.com/q9good/skarmar/tree/f0ea4430f601fcea1283da99c1e549af4ada386a/docs)
查看。原版本对部署状态的描述属于当时情况，后续进展应以当前交接为准。

## 导出范围

原快照中全部 `docs/`，另含根目录 README 和 AGENTS，共 17 个原始文件：
13 份 Markdown、1 份 JSON 任务清单、3 张合成验收截图。归档另附阅读索引和校验清单。
它不是云端文件系统的完整备份，不包含聊天、进程、未提交修改、数据库或凭证。

原任务另提到 `planning-20261018.zip` 私有环境启动配置备份，该文件未进入 Git，
本次未取回。不能把文档归档等同于该私有 ZIP；若后续需要，应从原任务单独安全导出。
