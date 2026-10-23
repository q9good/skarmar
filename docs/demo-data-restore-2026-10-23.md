# 追加演示测试数据并发布 T35 / T36

用户于2026-10-23要求先恢复一批测试数据，再部署线上。采用追加全新合成批次，不恢复整库、不清空现有数据，不复活T34已删除的旧ID。本机只有隔离自测快照；T34在线清理的完整私有记录没有上传，不能声称逐字段恢复了那十条。

## 可审查批次与执行顺序

[固定批次](../tools/fixtures/demo-20261023.json)全部标记为“演示测试1023”，业务日期固定2026-10-23，另有2026-09-30的课程用于上个自然月课时统计。包括6目标（1长期、2中期、3短期）、1方法、2家庭干预（1已完成／1计划草稿）、2课程、2困难、2经验，共15条业务记录；困难／训练经验由原业务接口生成，老师课程选项另计。保留未覆盖短期方向，便于对比规划的家庭／机构／合计视图。

两阶段通过**普通应用认证API**执行，固定UUID及请求操作ID：

1. 构建／测试通过后，用现有已审批账号登录原 `https://skarma.vip` / `default`，追加旧版支持的14条基础业务记录（课程暂为原文字段）。所有ID先检查；重复运行不重复新增；有独立编辑、删除、停用／改名则拒绝覆盖。没有DELETE、直接SQL、账号重建、DB替换或关闭认证。
2. 原发布脚本核对原Worker、DO、来源和域名，上传／激活新版并完成公开验收。
3. 在已激活新版上为两条合成课程补齐目标、实际方法关联，其中一条生成来源困难，最终15条。再核对全部显式关联；此前普通家庭跟进不被重新生成。

每一步业务写入保留服务器版本检查；整个批次由多次HTTP组成，**不是跨请求的单一事务**。中途失败会保留已成功的追加记录，原有数据不回滚；修复后以同一批次重试可续接。若正式发布成功、课程关联阶段失败，应报告“已发布，数据关联待补齐”，不能重新上传来碰运气。

## 仅差应用登录的安全输入

Cloudflare Token用于代码上传／激活，不是应用Cookie，不能据此绕过应用认证。当前Cloud任务没有站点登录会话，也无法从另一台电脑取出已登录Cookie。现有GitHub Secrets仍保留：`CLOUDFLARE_API_TOKEN`、`SKARMA_WRANGLER_TOML`，无需更换。

在 [GitHub → Repository Actions Secret](https://github.com/q9good/skarmar/settings/secrets/actions/new) 新增：

- Name：`SKARMA_TEST_LOGIN`
- Secret：JSON，填入**现有已审批的 skarma.vip 账号及密码**。普通成员即可，不强制使用管理员：

```json
{"username":"你在skarma.vip的账号","password":"该账号的密码"}
```

只在GitHub的安全Secret字段填写，不放聊天、Issue、Git或普通仓库变量。脚本仅将其发送至固定 `https://skarma.vip/api/auth/login`，Cookie仅存内存，结束时注销这次会话；不会修改密码或其他人的会话。成功后可删除这项临时Secret；原有两项发布Secrets保留。

GitHub当前集成读取该Secret元数据403，所以不能据此判断Secret是否已存在，更不能读取加密值。真实运行的第一步会检查是否配置，缺少即在业务写入与发布前停止。

## 触发与证据

已审查代码提交／推送后，最后一条明确发布提交同时包含以下两个标记（不得在未决定执行时使用）：

```bash
git commit --allow-empty -m 'Restore reviewed demo batch and release application [restore-demo:20261023] [deploy-cloudflare]'
npm run sync:push
```

普通push仍只运行guard；只补验模式禁止恢复数据。发布期间保持分支不动。工作流先检查三项Secrets，再构建／测试，恢复基础记录，发布并验收，最后补课程关联。公开artifact仅脱敏的release receipt及两个restore receipts；不会上传状态响应、账号、Cookie、完整环境配置或数据库。

本轮本机验证：9项合成恢复guard、21项原发布guard、YAML结构／固定Actions提交／阶段顺序检查通过。`npm run test:demo-restore`在原生SQLite与真实Rust Worker/Miniflare DO隔离空间通过：有认证API、保留既有对照目标、基础14条→完整15条、重复两阶段零新增／不改状态、独立编辑后拒绝覆盖、两后端重启全状态对账。未把这些检查称为线上恢复或部署成功。T35/T36应用验收提交仍为5eaa910，部署版本以真实release receipt为准。
