# 六项试用反馈整改发布（2026-11-08）

状态：**已上线并通过公开检查**。入口：**https://skarma.vip**。

用户明确要求部署`a11913bc34f58b30e9674ac8c8c2be5edf901c9f`，功能、逐项验收与12图见[整改清单](review-fixes-2026-11-08.md)。

## 发布验收逐项结果

| 编号 | 要求 | 实际结果 |
| --- | --- | --- |
| D01 | 精确源码、配置和发布保护 | 应用a11913b，触发f852dc8；工作区干净、远端相同；37项发布保护、9项示例保护、206项SHA通过；现有配置和目标经runner核对 |
| D02 | 真实上传和激活 | [run42483621978](https://github.com/q9good/skarmar/actions/runs/42483621978) validate／release均success，上传并激活`d4e115b1-8270-42c6-97e7-112ac119d2fd`，脱敏回执deployed_and_verified |
| D03 | 构建匹配、接口和代际 | runner与云端独立公开检查均通过，首页／JS匹配、健康／匿名／来源、DB启动正常，发布前后formal-20261101完成标记保持 |
| D04 | 正式数据保留、维护禁用与证据 | 清理／初始化／示例恢复／旧目标删除及课程补关联和相关演练全部skipped，原绑定／变量／runtime／路由保持；两份脱敏结果登记，208项SHA通过 |

## 版本和回执

- 应用源码：`a11913bc34f58b30e9674ac8c8c2be5edf901c9f`。
- 触发／部署源码：`f852dc8df0b19245bb402d8796cf5ab090af5560`，仅在已验收应用上增加发布记录。
- 原版本：`7df61582-5c77-49ab-b90b-ecfe7cf5d4f0`。
- 上传并实际激活：`d4e115b1-8270-42c6-97e7-112ac119d2fd`，100%流量。
- [原始脱敏回执](artifacts/deployment/cloudflare-release-20261108-receipt.json)／[独立公共结果](artifacts/deployment/cloudflare-release-20261108-independent-checks.json)。

runner重新执行类型检查、Web生产构建、182项Rust工作区测试及Wasm Clippy，核对原生产配置和已运行版本，先构建候选并再次比较存储绑定、变量和运行设置、域名路由，不变后才激活。私密配置清理步骤成功。分支保持触发SHA至run结束，之后才记录与推送结果。

## 数据与验收范围

保留原Worker、SkarmaSpace／SKARMA_DB／default、skarma.vip与精确来源、账号和全部正式业务／历史／冻结引用、v7、formal-20261101及草稿／离线队列键。应用没有Rust／schema变更，只有经T76逐项合成验证的客户端修复与图形布局。未清库、不初始化、不恢复示例、不重跑旧目标删除，不重设管理员或新建资源。

独立检查确认：

- 首页只去掉一个严格匹配的CF Analytics标签后与本机本次构建一致，JavaScript逐字节相同。
- health200／status ok；匿名身份与状态401、未知API404；合法来源退出200，非法来源或缺请求标记403。
- 随机未签发会话退出完成PRD数据库启动检查，不注销家人实际会话。
- 发布前后formal-20261101／completed:true保持。

没有登录真实账号、读取或逐条比对生产业务、写入线上验收样本。数据保留依据是普通发布、原存储及配置不变、稳定数据代际和维护入口全部关闭，不宣称逐条核对真实数据。六项业务流程依据本轮T76的90客户端、类型／Web和12组浏览器、最终5组复验；这些和本次runner／公开验收分别记录，没有把旧结果或dry-run当作上线证据。

12图及checks保留生成时deployed:false，并非线上截图。继续使用既有GitHub Secrets；Cloud原生凭据T12仍独立受阻，Opus／真机／读屏及implementationCommit不变。
