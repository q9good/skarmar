# 可同步的示例截图

日期：2026-10-18。以下三张图片来自专用临时数据库上的自动浏览器验收，已确认仅含空白登录页或合成验收内容。对应应用提交为 `ac7c1f5`。它们随 Git 同步，不能作为用户真机或真实主机部署验收。

| 图片 | 内容 |
| --- | --- |
| [login.png](login.png) | 空白登录页 |
| [pathways.png](pathways.png) | 合成途径目录与手机导航 |
| [planning-charts.png](planning-charts.png) | 合成训练数据的环形图 |

校验值见 `../../workspace-state.json`，运行 `npm run sync:status` 校验。新截图需先检查是否包含个人资料、真实训练内容或凭证，再更新对应校验值；不从 `.local/screenshots` 自动批量复制。
