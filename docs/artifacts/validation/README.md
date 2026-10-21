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

编辑专项自测的合成截图（本地Worker/DO）：

| 图片 | 内容 |
| --- | --- |
| [edited-goal-history.png](edited-goal-history.png) | 修改后的目标标准及保留的新旧版本 |
| [edited-experience-bindings.png](edited-experience-bindings.png) | 修改后的经验、目标／方法绑定及验证依据 |

当时通过项与缺失项见[编辑与删除自测](../../edit-delete-self-test.md)，后续已由T30补齐删除和已完成训练修订。

管理员删除与普通成员修订的合成截图（本地Worker/DO）：

| 图片 | 内容 |
| --- | --- |
| [member-training-revision.png](member-training-revision.png) | 普通家人成员修改已完成训练，页面不提供删除入口 |
| [admin-referenced-goal-delete.png](admin-referenced-goal-delete.png) | 管理员二次确认及被训练引用目标的拒绝提示 |
| [deleted-training-retains-experience.png](deleted-training-retains-experience.png) | 原训练删除后保留经验、绑定、依据及来源删除说明 |

规则及双后端验收见[管理员删除与训练修订](../../record-permissions.md)。

2026-10-22 保存反馈修复的合成截图：

| 图片 | 内容 |
| --- | --- |
| [training-saving.png](training-saving.png) | 隔离示例空间中暂停真实保存响应，页面显示正在保存，不出现结果未确认提示 |

验证与发布边界见[保存反馈修复](../../ui-feedback-fixes-2026-10-22.md)。
