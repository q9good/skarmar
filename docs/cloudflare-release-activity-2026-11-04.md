# 活动式计划与全目标总览发布（2026-11-04）

状态：**已上线并通过公开检查**。入口：**https://skarma.vip**。

用户明确要求直接部署刚完成的T68。家庭计划只写活动，目标／方法及知识为限定路径的只读参考；总结再明确选择实际覆盖，原计划快照不由事后选择倒填。课记结果页移除新增／续填；目标页增加覆盖／状态环图、完整图例和下钻，保留逐目标实际测量。范围与8图见[设计与验证](activity-plan-and-goal-overview-2026-11-04.md)。

## 实际发布结果

| 项目 | 结果 |
| --- | --- |
| 应用源码 | `895a665898e1a518bbabb851f3689233950a270b` |
| 触发／部署源码 | `25ca4d9359ad3438ae7d421d36f64cc9ab4afcad` |
| Actions | [run41766466317](https://github.com/q9good/skarmar/actions/runs/41766466317)，validate／release均success |
| 原版本 | `63af67ad-78c2-4803-89cf-525c35bcb736` |
| 上传并实际激活 | `10ff3a07-ee68-4305-bc41-bb58a27f0812` |
| 模型／结构 | `prd_v1`／v7，原存储保持 |
| 回执 | [deployed_and_verified](artifacts/deployment/cloudflare-release-activity-20261104-receipt.json) |

runner重新执行类型检查、Web生产构建、Rust工作区测试及Wasm Clippy，核对原目标后上传候选。比较存储绑定／变量／运行设置与路由，确认保持后才激活100%流量。原Worker、SkarmaSpace/SKARMA_DB/default、skarma.vip和精确来源均保留；临时私密配置清理成功。发布期间分支保持触发SHA，run结束且独立检查通过后才收尾文档。

## 数据与验证

仅普通发布标记；清理、初始化、演示恢复、合成课程补关联及相关演练全部skipped。没有schema迁移，`formal-20261101`及本机草稿／队列键不变。账号、全部正式业务、历史与来源关联保持；旧记录不被后台重写，固定原计划快照的规则作用于之后显式保存。

发布前32项发布保护、9项示例工具保护和167项产物SHA通过；应用源摘要与已审截图一致。T68开发阶段已通过75客户端、171Rust、类型／Web、原生／Wasm Clippy、Worker构建、完整双运行环境／重启无SKIP及30组Web；这些与本次runner重跑的检查分别记录。

runner及[云端独立公开检查](artifacts/deployment/cloudflare-release-activity-20261104-independent-checks.json)均通过：

- 首页只排除严格匹配的一个CF Analytics标签后与本次构建一致；实际JavaScript逐字节相同。
- health200／status ok，匿名身份与状态401，未知API404；合法Origin退出200，非法Origin或缺请求标记403。
- 未签发随机会话的退出检查确认PRD数据库启动成功，不改变家人现有会话。
- 发布前后正式启用标记均保持`formal-20261101/completed:true`。

没有登录真实账号、读取业务内容、逐条比对真实记录或写入生产验收样本，也没有清库、补种、新建资源或重设管理员。数据保留依据是普通发布、原存储不变、稳定代际与关闭全部显式清理入口，并非业务内容逐条检查。两份脱敏结果登记后共169项产物SHA通过。

8图、HTML及其机器结果保留生成时的`deployed:false`，不改写历史开发证据。原Cloud环境T12仍是独立事项，本次沿用Actions安全凭据；Opus／真机／读屏及`implementationCommit`不变。
