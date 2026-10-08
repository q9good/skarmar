# 腾讯多维表格核验

更新于 2026-10-08。本次只读核验由用户明确授权；附件中的旧操作指令作为历史资料。

## 当前证据

附件最终快照于 2026-10-07 21:24:24（Asia/Shanghai）采集：11 目标、55 训练、0 正式目标明细、
52 困难、0 经验、15 使用说明。已检查字段、视图和记录页的完整性，并审计引用关系。
详情与迁移规则见 [产品方案](product-plan.md#7-迁移基线与边界)。私有核对报告存于
`/workspace/private/tencent/baseline-audit.json`，源记录 ID 和正文不进入 Git。

**尚未完成线上核验。** 无凭证网络探测在代理 CONNECT 阶段返回 403；当前运行策略没有
`docs.qq.com`，尚未进入 token 验证。因此没有证据判断 token 是否有效，也没有当前线上数据副本。
已在云环境配置草案添加 `docs.qq.com`。草案保存不会立即改变运行策略，需在环境设置评审并保存／发布
使其生效；之后检查运行策略并重试。保留环境代理与 CA，不通过关闭代理或 TLS 校验绕过限制。

## 只读 MCP 客户端

当前镜像已有 MCP Python SDK 1.29.0。本地客户端连接官方 `https://docs.qq.com/openapi/mcp`，
读取工具目录并支持单次只读工具调用；没有安装到 Codex 工具列表。
其他镜像可在虚拟环境按 `tools/tencent-requirements.txt` 安装依赖。

网络放行后，在 worktree 根目录执行：

```bash
python tools/tencent_readonly.py catalog \
  --credential-file /workspace/research-inputs/ASD/ASD/secret.txt \
  --output /workspace/private/tencent/catalog.json
```

先检查私有工具目录中的名称、说明和参数 schema，再选择确切的查询接口。客户端仅接受
`query_` / `get_` / `list_` 开头且没有修改动作的名称；名称过滤不代替人工核对工具语义。
具体工具名称和参数在成功读取目录前保持未定。

```bash
python tools/tencent_readonly.py call \
  --credential-file /workspace/research-inputs/ASD/ASD/secret.txt \
  --tool '<已核对的只读工具名称>' \
  --arguments-file /workspace/private/tencent/arguments.json \
  --output /workspace/private/tencent/response.json
```

token 直接作为 Authorization，不增加 Bearer 前缀；凭证只从仓库外读取，不打印请求头或异常详情。
响应必须保存在 Git checkout 外，文件权限 0600。记录分页需保存所有页并验证游标推进，不能把单页
响应当作完整导出；单次调用工具没有自动汇总记录页。

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

## 下一次核验的完成条件

1. MCP 初始化成功，读取当前目录并确定工具的只读语义。
2. 文件与表清单可读；字段、视图、记录全部分页完成且数量一致。
3. 核对目标状态／归档两组选择项、父级关系、合并后的目标关系及困难的多个来源。
4. 对比附件快照，列明新增／修改／删除和悬空引用；无法读取自动化配置时注明人工核验项。
5. 保留私有原始副本与字段映射；公开方案仅记录结构、计数、差异类型和结论。

以上完成前，“已核对附件”与“已读取线上表格”分别标记。
