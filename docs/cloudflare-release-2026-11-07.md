# 目标方法关系图与课堂回看发布（2026-11-07）

状态：**已上线并通过公开检查**。入口：**https://skarma.vip**。

用户明确要求部署T72–T74。应用实现、范围与6张合成手机截图见[设计与验证](object-map-and-classroom-review-2026-11-07.md)。

## 实际发布结果

| 项目 | 结果 |
| --- | --- |
| 应用源码 | `d71b1ecb3404170220d41a46386e22d855b315c1` |
| 触发／部署源码 | `a0d930ec0ef34a7f7c7777e6da46ce73ac657569` |
| Actions | [run42419768909](https://github.com/q9good/skarmar/actions/runs/42419768909)，validate／release均success |
| 原版本 | `965b6832-9b54-4a53-8711-df0ede5c0e3d` |
| 上传并实际激活 | `7df61582-5c77-49ab-b90b-ecfe7cf5d4f0` |
| 模型／结构 | `prd_v1`／v7，原存储保持 |
| 回执 | [deployed_and_verified](artifacts/deployment/cloudflare-release-20261107-receipt.json) |

runner重新执行类型检查、Web生产构建、182项Rust测试及Wasm Clippy，核对现有目标、原存储绑定、运行设置、变量与路由，上传候选后再次比较，保持不变才激活100%流量。原Worker、SkarmaSpace／SKARMA_DB／default、skarma.vip、精确来源和既有私密配置保留。发布期间分支保持触发SHA，运行结束后才收尾文档。

## 数据保留与验收边界

仅普通发布；清理、初始化、示例恢复、合成课程补关联及对应演练全部skipped。未重跑T71两项目录删除。账号、全部正式业务、旧审计和冻结来源保留，v7／formal-20261101及草稿／离线队列键不变。两个回执查询索引幂等创建，不清除旧观察；新补充不再追加用户修改列表。方法优先级为可选字段，旧客户端省略字段时保留新值。

发布前当前应用源码通过182Rust、87客户端、类型／Web、双Clippy、Worker构建、完整SQLite／Rust DO／重启与13组浏览器合成验收；本轮另跑37项发布保护、9项示例工具保护、191项SHA。开发检查与本次runner检查分别记录，没有把dry-run当作上线。

runner与[云端独立公开检查](artifacts/deployment/cloudflare-release-20261107-independent-checks.json)通过：

- 首页仅排除一个严格匹配的CF Analytics标签后与构建一致，公开JavaScript逐字节相同。
- health200／status ok，匿名身份与状态401，未知API404；合法来源退出200，非法来源或缺请求标记403。
- 随机未签发会话的退出请求完成PRD数据库启动检查，不注销任何家人会话。
- 发布前后正式代际标记均保持formal-20261101／completed:true。

未登录真实账号、读取或逐条比对生产业务、删除记录、恢复示例、写入业务验收样本、新建资源或重设管理员。数据保留依据为原存储不变、稳定代际、关闭全部显式清理入口及当前双运行环境合成契约，不声称逐条核对真实数据。两份脱敏结果加入后193项产物SHA通过。

6图与checks保留生成时deployed:false。原Cloud环境T12、Opus独立验收、真机／读屏及implementationCommit维持原状态；Actions可发布不等于Cloud原生凭据问题已修复。
