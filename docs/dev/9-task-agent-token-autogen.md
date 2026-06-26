# Agent token 自动生成任务记录

> 日期：2026-06-26
> 级别：L2
> 状态：已完成

## 背景

用户希望去掉 agent 配置中的 labels 和 token 相关字段，token 默认由系统自动生成，减少安装后必须修改的配置项。

## 实现

- agent 启动时如果 `[agent].token` 未配置，会在 `<core.data_dir>/agent.token` 生成并保存 token。
- Unix 平台下 token 文件权限设置为 `0600`。
- server 不再要求 `[agent.<name>]` 配置 token 或 labels，只保留 `enabled` 控制。
- 未预置的 agent 首次连接时自动加入 server 运行时 agent 列表。
- server 在 agent websocket hello 时登记 token，源码包和升级包 HTTP 下载继续使用 agent name/token header 校验。
- Web UI Submit Build 只支持勾选目标 agent，不再支持 labels 输入。
- 默认配置、测试配置、README 和概要文档移除用户需要配置 token/labels 的说明。

## 兼容说明

- 老配置中的 `[agent].token` 仍会被兼容读取并优先使用，避免升级后立即切换 token。
- 数据库中的 run `labels_json` 字段暂时保留，新增 run 写入空 labels，避免引入迁移。

## 验证

- `cargo fmt --check`
- `make test`
- Web UI 内嵌脚本 `node --check`
- `make deb`
- 解包检查 `/etc/buildsvc/buildsvc.ini` 不包含 `token =` 或 `labels =`
