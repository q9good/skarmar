# 腾讯多维表格核验

更新于 2026-10-09。本次只读核验由用户明确授权；附件中的旧操作指令作为历史资料。

## 当前证据

附件最终快照于 2026-10-07 21:24:24（Asia/Shanghai）采集：11 目标、55 训练、0 正式目标明细、
52 困难、0 经验、15 使用说明。已检查字段、视图和记录页的完整性，并审计引用关系。
详情与迁移规则见 [产品方案](product-plan.md#7-迁移基线与边界)。私有核对报告存于
`/workspace/private/tencent/baseline-audit.json`，源记录 ID 和正文不进入 Git。

**线上读取已成功。** 用户调整网络设置后，运行配置为 unrestricted，MCP 初始化和所有本次
查询均成功。此前 CONNECT 403 的阻塞已解除；token 已能读取目标文件。
全程保留环境代理、CA 和 TLS 校验。

采集窗口为 **2026-10-09 07:10:28—07:10:37（Asia/Shanghai）**。6 张表的字段／视图／记录均
完整分页、无重复记录 ID，数量与 total 一致。表／记录 ID、字段定义、视图配置、字段值与附件
逐项比对均一致。完整查询是一个时间窗口内的副本，不代表腾讯接口提供跨表原子快照。

| 表 | 字段 | 视图 | 记录 |
| --- | ---: | ---: | ---: |
| 目标总表 | 26 | 6 | 11 |
| 每日训练记录 | 36 | 4 | 55 |
| 每日目标明细 | 17 | 2 | 0 |
| 困难跟进 | 11 | 5 | 52 |
| 成功经验库 | 13 | 3 | 0 |
| 使用说明 | 4 | 1 | 15 |

关系审计仍有 1,080 条原始字段引用、7 个指向旧表的字段、275 条旧表引用和 2 条已删除记录引用。
这些是现存关系的异常，不等于丢失同等数量的业务记录；正式迁移时保留原始证据并使用有效合并关系。

两条自动化摘要的名称、status=2、trigger_type=2 和动作数量已读取。摘要没有启停枚举说明、
具体条件与动作配置，尚不能验证其完整逻辑或运行可靠性。当前没有对应的自动化详情读取工具，
此项需要在腾讯文档界面人工核验，不调用写接口间接查看。

私有文件（均在仓库外、权限 0600）：

- `/workspace/private/tencent/catalog-live-20261009.json`：225 个工具的目录和 schema。
- `/workspace/private/tencent/snapshot-live-20261009.json`：原始响应页、完整汇总与自动化摘要。
- `/workspace/private/tencent/live-audit-20261009.json`：引用审计。
- `/workspace/private/tencent/live-comparison-20261009.json`：与附件的逐项比较。

## 只读 MCP 客户端

当前镜像已有 MCP Python SDK 1.29.0。本地客户端连接官方 `https://docs.qq.com/openapi/mcp`，
读取工具目录并支持单次只读工具调用；没有安装到 Codex 工具列表。
其他镜像可在虚拟环境按 `tools/tencent-requirements.txt` 安装依赖。

在 worktree 根目录执行：

```bash
python tools/tencent_readonly.py catalog \
  --credential-file /workspace/research-inputs/ASD/ASD/secret.txt \
  --output /workspace/private/tencent/catalog.json
```

先检查私有工具目录中的名称、说明和参数 schema，再选择确切的查询接口。客户端仅接受
`query_` / `get_` / `list_` 开头且没有修改动作的名称，也支持 `smartsheet.list_records` 等命名空间。
名称过滤不代替人工核对工具语义。本次检查并使用了 `smartsheet.list_tables`、`list_fields`、
`list_views`、`list_records`、`list_automations`。

```bash
python tools/tencent_readonly.py call \
  --credential-file /workspace/research-inputs/ASD/ASD/secret.txt \
  --tool smartsheet.list_tables \
  --arguments-file /workspace/private/tencent/arguments.json \
  --output /workspace/private/tencent/response.json
```

token 直接作为 Authorization，不增加 Bearer 前缀；凭证只从仓库外读取，不打印请求头或异常详情。
响应必须保存在 Git checkout 外，文件权限 0600。记录分页需保存所有页并验证游标推进，不能把单页
响应当作完整导出；单次调用工具没有自动汇总记录页。
本次完整分页导出使用被忽略的本地辅助脚本 `.local/tencent_snapshot.py`；它不是正式迁移工具。

## 附件关系审计

审计器接受附件最终快照的包装格式，不直接接受 MCP 单次响应。线上导出时先完整汇总每张表的
字段、视图和记录，并保留原始页与获取时间，再规范为同样格式。

```bash
python tools/snapshot_audit.py \
  /workspace/research-inputs/ASD/ASD/流程收束最终快照-2026-10-07.json \
  --output /workspace/private/tencent/baseline-audit.json
```

标准输出只有数量，不输出训练正文。报告区分旧表引用和已删除记录引用；不能据此构造虚假的
主／副目标或逐目标结果。当前只做审计，尚未实现正式数据导入。

## 完成项与剩余边界

1. MCP 初始化成功，读取当前目录并确定工具的只读语义。
2. 文件与表清单可读；字段、视图、记录全部分页完成且数量一致。
3. 核对目标状态／归档两组选择项、父级关系、合并后的目标关系及困难的多个来源。
4. 对比附件快照，列明新增／修改／删除和悬空引用；自动化具体配置已注明人工核验项。
5. 保留私有原始副本与字段映射；公开方案仅记录结构、计数、差异类型和结论。

以上数据读取与比对已完成，自动化详情和实际执行尚未验收。正式导入与切换前仍需重新取得最新
完整副本并核对，避免把本次时间点的结果当作后续持续不变的事实。
