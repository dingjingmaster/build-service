# 取消构建脚本超时限制调研

> 文档元数据
> - 文件编号：23
> - 文档类型：research
> - 文件路径：docs/dev/23-research-unlimited-script-runtime.md
> - 日期：2026-08-20
> - 需求级别：L3

## 背景

用户要求彻底去掉 `script_timeout_sec=7200` 这一限制，构建脚本不再因为运行时间达到固定阈值而被 buildsvc 终止。

## 现状证据

- server 创建 run 时把 `[server].script_timeout_sec` 写入 run 记录，并通过 `ServerToAgent::RunStart` 下发给 agent。
- agent 收到 run 后把 timeout 转为 `tokio::time::sleep`，脚本未结束且 sleep 到期时调用 `terminate_child` 并返回退出码 124。
- 实际失败 run `run_676941e4e6e04dfbbdc4fd46a73ba825` 运行 7201 秒，配置为 7200 秒，退出码为 124，日志尾部为 `Terminated/Error 143`。

## 需求边界

- 去掉构建脚本按时间自动终止的能力。
- 保留用户取消 run 时终止进程组的能力。
- 配置样例和文档中移除 `script_timeout_sec`。
- SQLite 历史 schema 中已有 `script_timeout_sec` 列，为避免破坏已有数据库，不做删列迁移；该列仅作为兼容遗留列，不再参与业务逻辑。

## 风险

- 配置格式和 agent/server WebSocket 协议字段发生变化，需要 server 与 agent 同步升级。
- 长时间或卡死脚本将一直占用 agent 并发槽，必须通过 UI Cancel 或外部进程管理止损。
- 旧配置文件中残留 `script_timeout_sec` 时，新版本会忽略该键。
