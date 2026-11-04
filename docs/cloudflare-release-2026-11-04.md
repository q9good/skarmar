# Nord配色与家庭计划调整发布（2026-11-04）

状态：**已上线并通过公开检查**。入口：**https://skarma.vip**。

用户明确要求部署当前版本。应用源码`54edea7702d1d32875f06d388e99d645ba4513bc`，
包含`f1dc03b`的家庭计划重排与必填安排检查，以及统一Nord衍生浅色方案。
准备做什么在最上面、目标和方法其次，关联知识去重按需显示；覆盖和历史参考按需展开。
草稿、角色笔记、直接总结、旧成功回执保留。
配色与截图见[Nord调研及13图评审](ui-palette-research-2026-11-03.md)。

## 实际发布结果

| 项目 | 结果 |
| --- | --- |
| 触发／部署源码 | `f841d4ce6719d02d2d829dc31d35e514f4805bba` |
| Actions | [run41749746282](https://github.com/q9good/skarmar/actions/runs/41749746282)，validate／release均success |
| 原版本 | `96b1cb22-c034-43a6-9f7e-b8c819703af6` |
| 上传并实际激活 | `63af67ad-78c2-4803-89cf-525c35bcb736` |
| 模型／结构 | `prd_v1`／v7，保持原存储 |
| 回执 | [deployed_and_verified](artifacts/deployment/cloudflare-release-20261104-receipt.json) |

runner重新执行类型检查、Web构建、Rust测试及Wasm Clippy，再核对原目标、上传唯一候选，
比较存储绑定／变量／运行设置和路由，确认保持后才激活100%流量。
只有普通发布标记：清理、模型初始化、演示恢复及合成关系补齐步骤全部skipped。
临时私密配置清理成功；没有提取、修改或公开Secrets。
发布期间共享分支保持在触发SHA，run结束后才提交本记录，不再次部署。

## 数据与验证

保留`skarma-web`、`SkarmaSpace/SKARMA_DB/default`、`skarma.vip`、精确来源、账号、
全部正式业务数据、历史、关联、v7结构，以及`formal-20261101`和本机草稿／队列键。
此次没有schema迁移，后端仅增加计划保存时安排非空校验。
成功旧回执重放、旧记录读取、直接总结和总结草稿等已在开发回归验证。

发布前本轮32项发布保护、9项演示工具保护和155项产物SHA通过，配色源摘要与评审一致。
T65此前通过169项Rust、双运行环境／重启、29组Web；T66最后配色通过类型／Web构建、
68项客户端、12组Web及最终计划／覆盖补验。这些开发检查与本次runner检查分别记录。

runner与[云端独立公开检查](artifacts/deployment/cloudflare-release-20261104-independent-checks.json)均通过：

- 首页只排除一条严格匹配的CF Analytics注入后与构建一致，实际JavaScript逐字节匹配。
- health200／status ok，匿名身份与状态401，未知API404。
- 无账号Cookie的合法Origin退出200，非法Origin或缺请求标记403。
- 从未签发的随机会话退出检查确认PRD数据库启动成功，不影响家人现有会话。
- 正式启用完成标记在发布前后保持`formal-20261101/completed:true`；此次未执行清理。

没有登录真实账号、读取业务内容、逐条比对真实记录或写入生产验收样本；数据保留由普通发布、
原存储绑定不变、稳定代际及显式清理关闭共同保证。没有清库、补种、新建资源或重设管理员。
两份脱敏上线结果登记后共157项产物SHA通过。

当前13图、HTML及其机器结果继续保留生成时的`deployed:false`，不改写历史测试证据。
原Cloud环境T12凭据问题未修复，实际发布通过已有Actions安全凭据完成；
Opus、真机、完整读屏与正式验收指针`implementationCommit`保持原待办／值。
