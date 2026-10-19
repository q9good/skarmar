# 可同步的示例截图

日期：2026-10-19。这些图片来自专用临时数据库的自动浏览器验收，已逐张检查，仅包含空白登录页或合成内容，不代表真实手机网络与生产发布验收。

| 图片 | 内容 |
| --- | --- |
| [login.png](login.png) | 历史空白登录页，未改动 |
| [pathways.png](pathways.png) | 本轮方法目录、关联目标置前及新导航 |
| [planning-charts.png](planning-charts.png) | 本轮计划内的均衡提示与环形图 |
| [page-workflows-today.png](page-workflows-today.png) | 本轮今天页、长期关注及机构／家庭干预说明 |

本轮应用对应提交见 `../../workspace-state.json` 的 implementationCommit。截图校验值记录在该清单，执行 `npm run sync:status` 验证。不从 `.local/screenshots` 自动批量复制；真实业务截图、账号、凭证及原始日志不能进入本目录。

关联专项自测截图来自全新临时空间中的Rust Worker / Miniflare DO API。每张只含明确标注的合成资料：

| 图片 | 内容 |
| --- | --- |
| [relation-goal.png](relation-goal.png) | 目标显示关联方法和两类经验 |
| [relation-method.png](relation-method.png) | 方法显示两类关联经验 |
| [relation-experience.png](relation-experience.png) | 经验显示目标、方法和来源训练入口 |
| [relation-training.png](relation-training.png) | 训练显示目标快照与成功经验 |

完整核对矩阵见 [关联自测](../../relationship-self-test.md)。
