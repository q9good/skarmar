# 家庭总结与提醒整改普通发布（2026-11-09）

状态：**已上线并通过公开检查**。入口：**https://skarma.vip**。
用户明确授权四项整改完成后直接部署；应用`2c0821b18551f2d54469b016fc42de762a3aaa0c`，逐项结果、失败基线及9图见[B01–B04](summary-and-reminders-2026-11-09.md)。

## D01验收

| 步骤 | 实际结果 |
| --- | --- |
| 来源与保护 | 应用2c0821b，普通触发9da2d08；41发布保护、9示例保护、232项产物SHA先通过，分支无并行发布且保持触发SHA至run结束 |
| 原配置与真实激活 | [run42631081552](https://github.com/q9good/skarmar/actions/runs/42631081552) validate／release均success，原绑定／变量／runtime／路由比较不变后切换100%，上传与实际激活`bda6d08a-5f09-4b22-80b5-02a07633829c`，deployed_and_verified |
| 公开检查 | runner及云端独立检查通过：首页／JS与构建匹配、health200／ok、匿名401／未知404、合法来源200／非法或缺请求标记403、随机未签发会话PRD启动完成 |
| 正式数据保持 | 发布前后formal-20261101完成标记一致；清理／初始化／示例恢复／旧目标删除／课程补关联及对应演练全部skipped，私密配置清理成功；两份脱敏结果后234项SHA |

- 应用源码：`2c0821b18551f2d54469b016fc42de762a3aaa0c`。
- 触发／部署源码：`9da2d080deb6df852d3c8c797df67f1013a17023`（只在验收应用上增加发布说明）。
- 原版本：`64921193-38e0-4d50-98d7-f223a44d0eb1`；上传并实际激活：`bda6d08a-5f09-4b22-80b5-02a07633829c`。
- [脱敏回执](artifacts/deployment/cloudflare-release-summary-20261109-receipt.json)／[独立公开检查](artifacts/deployment/cloudflare-release-summary-20261109-independent-checks.json)。

runner实际重跑类型检查、Web构建、185Rust、Wasm Clippy及无凭据的Worker release构建，再读取原生产配置、构建并比较候选绑定与路由后激活。分支在run完成前保持部署源码，之后才登记发布结果。

## 验收与数据边界

保留原Worker、SkarmaSpace／SKARMA_DB／default、skarma.vip与精确来源、账号和全部正式业务／历史／冻结来源、schema v7、formal-20261101及原草稿／离线队列键。没有迁移、清理、自动补写旧知识、恢复样本、重建资源或重设账号。普通发布不带任何历史维护标记。

业务功能依据本机合成数据：99客户端、类型／Web、SQLite／真实Rust DO完整合约和重启、12组相关浏览器；最终核验保护后针对6组复验。已复现的是基线经验漏选阶段422且错误在屏外，明确阶段后同一基线能正常保存；不声称复现用户生产数据库丢失。两类知识的真实提交、草稿／完成、补充修订、原回执、多人、跨页显示、共享503和200缺行核验均分别检查。

独立公开检查只去掉严格匹配的一个CF Analytics标签再比较首页，JS逐字节一致；随机未签发会话只验证PRD启动，不注销家人会话；正式数据代际前后保持。没有登录真实账号、读取／逐条对账生产业务或写入线上验收样本。数据保留依据是原存储及配置保持、稳定代际和维护全部关闭，而非声称逐条查看真实记录。

截图均已检查仅合成，保留生成时deployed:false，不改作线上业务证据。T12仍独立受阻；继续沿用原GitHub Secrets发布通路，Opus／真机／读屏和implementationCommit不变。
