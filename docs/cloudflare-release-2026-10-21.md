# 2026-10-21 Cloudflare 正式入口更新

## 发布结果

用户要求先同步最新 commit 再发布，并明确当前数据均为测试数据、无需备份。
本次使用 Mac 上已存在的 Cloudflare OAuth 登录完成发布，不依赖尚未恢复的 Codex Cloud 环境。

| 项目 | 本次结果 |
| --- | --- |
| 分支 | `feat/tencent-informed-web` |
| 发布源码 HEAD | `f0a4e588fb2b97289ca71e88c3a2586c32f278bd` |
| 应用验收对应提交 | `ffdd0d0ddd67d44b8d169441f5f1f0dbafe5af94` |
| 原 Worker | `skarma-web` |
| 新版本 | `7398e1af-496e-415e-b113-7c6d0b68a45c` |
| 上一版本 | `eb0b5584-2ae8-4723-bc8a-d99aec0cecc6` |
| 入口 | [skarma.vip](https://skarma.vip/)；原 workers.dev 入口保留 |
| 内容 | T27 六页流程、T30 管理员删除／家人训练修订、T31 取消密码最小长度 |

本轮没有修改应用源码。后续文档提交只记录发布结果，不改变以上实际上传的代码。

## 同步与配置核对

1. 工作区干净时，以现有同步脚本安全获取并快进共享分支，确认远端最新 HEAD 为 `f0a4e58`。
2. 原云端任务确认实际 HEAD 相同、无未提交或未推送应用改动，发布期间保持分支不动。
3. `wrangler whoami` 确认 Mac 既有 OAuth 有效，账号与本机生产配置一致；没有生成或读取新密钥。
4. 读取旧版本详情，保存版本 ID，并与新版本元数据逐项比较：原 DO 命名空间、
   `SkarmaSpace`、`SKARMA_DB`、`SKARMA_SPACE=default`、生产／认证开关及两个精确来源全部相同。
5. 沿用被 Git 忽略的 `deploy/cloud-worker/wrangler.toml`，其中包含 `skarma.vip` Custom Domain。
   不使用根目录开发配置或含占位符模板进行正式发布。

## 实际构建和发布命令

从仓库根目录运行；代理和工具路径按本机实际配置，不复制凭证：

```bash
npm run web:build
export PATH="$PWD/.local/worker-tools/bin:$PATH"
./node_modules/.bin/wrangler deploy \
  --config deploy/cloud-worker/wrangler.toml --dry-run \
  --outdir .local/cloud-worker-dry-run-20261021
./node_modules/.bin/wrangler deploy \
  --config deploy/cloud-worker/wrangler.toml \
  --message 'Release ffdd0d0: page workflows, record permissions and password rules; source f0a4e58'
```

本次 Mac 对联网命令使用现有 `HTTPS_PROXY`，保留 TLS 校验。正式命令退出 0，
返回 Uploaded skarma-web、Deployed skarma-web triggers 和上述新版本 ID。
代码 gzip 约 1490.64 KiB，启动时间 7 ms。原始日志和实际配置仅保存在本机忽略目录。

## 本轮线上验收

| 检查 | 实际结果 |
| --- | --- |
| 首页和实际引用 JS | 200；线上 JS SHA-256 与本次本地构建相同 |
| JS 构建标识 | `index-dbf2b8b633df491880a5a9d074b19173.js` |
| `/api/health` | 200，`status=ok` |
| 匿名 `/api/auth/me`、`/api/state` | 401 |
| 未知 API | 404 |
| 合法域名来源、带请求标记的匿名 logout | 200 |
| 非法来源，或缺少请求标记 | 403 |
| 已有浏览器登录会话 | 正常进入新版今天页；长期关注置顶，课程与家庭干预入口可见 |
| 刷新 | 发布切换时曾短暂显示自动刷新失败；手动刷新成功，后续自动同步时间继续更新，无错误提示 |
| 原数据与账号 | 仅通过原实例的只读聚合查询核对，发布前后各业务表与账号数量一致 |
| 数据迁移 | 迁移版本为 9；不重建对象、账号或业务库 |

按用户要求跳过备份；只读数量记录不能当作备份或逐条数据对账。本轮未创建线上测试账号、
未新增／修改／删除业务记录，未重新执行全部应用测试。此前 72 项 Rust、双后端合约、
手机浏览器等为云端在 `ffdd0d0` 对应代码完成的验收，详见 [原验收记录](registration-password-release.md)。

## 回退和继续开发

上一版代码 ID 已保留。需要回退时选择原 Worker 的兼容版本；代码回退不回退数据库，
不得删除对象来清空迁移，也不将业务 Excel 或数量记录当作整库恢复包。本次未演练生产恢复。

开发继续使用共享分支，更新前先安全同步。实际发布记录与任务状态随本次文档提交推送，
并由原 Codex Cloud 任务确认实际拉取到的 SHA，不能只靠更新本机状态文件声称云端已同步。

云环境故障反馈已提交、没有工单编号，但环境发布尚未恢复。T32 应用发布已完成，
T12 环境配置仍受阻；继续 Cloud 自动发布前须验证 Token 真实绑定与 Wrangler 认证。
不因本次 Mac 发布成功而重试锁定草案或重新创建凭证。参见 [故障记录](environment/cloudflare-token-publish-investigation.md)。
