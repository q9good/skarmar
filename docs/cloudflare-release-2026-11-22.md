# 11 月 22 日手机操作修复发布

本次普通发布包含 T82 至 T84 经手机视窗走查的修复。审查提交为 `5cf2925`，应用提交为 `9271212`。目标是现有 skarma.vip、skarma-web 和 default 空间。正式账号、业务数据及草稿使用原存储。

|编号|发布验收|
|---|---|
|T85-1|从已审查的分支触发普通发布，核对构建、手机流程和实际激活版本。|
|T85-2|核对登录入口、完整公开脚本、健康接口、匿名访问及正式数据代际，记录发布回执。|
|T85-3|确认示例恢复、数据清理和账号初始化步骤未运行，更新任务状态。|

操作审查见[手机走查记录](mobile-operation-review-2026-11-20.md)。

## 发布结果

|编号|实际结果|
|---|---|
|T85-1|GitHub Actions 两项作业通过。Web、Rust 和三组发布前手机流程均通过。普通发布提交为 `479aff6`。现有 Worker 从 `f0416444-6ef3-4231-b4ac-bdb7c3f4c7aa` 切换至 `bb1028d5-a43d-416b-a347-52c9c6e0f0bb`，上传与激活版本一致，回执为 `deployed_and_verified`。|
|T85-2|11 月 22 日 09:16（北京时间）独立复核首页和健康接口均返回 200。首页仅含预期的 Cloudflare Analytics 附加标签，公开脚本 1,197,287 字节与本次构建逐字节一致。匿名访问、精确来源和数据库启动检查均通过。|
|T85-3|发布器核对了原有路由、存储绑定与运行配置。发布前后正式数据代际均为 `formal-20261101`。示例恢复、初始化、清理及选定目标移除步骤全部跳过。公开检查未登录家庭账号或读取、写入业务记录。|

[Actions 运行](https://github.com/q9good/skarmar/actions/runs/44970018935) · [脱敏发布回执](artifacts/deployment/cloudflare-mobile-workflows-20261122-receipt.json) · [独立公开检查](artifacts/deployment/cloudflare-mobile-workflows-20261122-checks.json)
