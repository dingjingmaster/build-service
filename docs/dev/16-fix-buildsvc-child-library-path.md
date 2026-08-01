# buildsvc 子进程动态库环境污染修复记录

> 文档元数据
> - 文件编号：16
> - 文档类型：fix
> - 文件路径：docs/dev/16-fix-buildsvc-child-library-path.md
> - 完成日期：2026-08-01
> - 需求级别：L2

## 1. 问题定义

- 问题现象：目标机 buildsvc 主服务显示 active，但 `run-build.sh`、bash 和 systemctl 子进程段错误，构建和远程关机失败。
- 预期行为：buildsvc 使用随包动态库启动；它启动的系统命令使用目标机系统动态库并正常运行。
- 影响范围：Linux 打包版 agent 的构建脚本、终端和系统命令。
- 不包含：不修改远程机器配置，不调整 glibc，不改变 buildsvc 协议。

## 2. 证据与根因

- 复现方式：目标 Ubuntu 16 机器上，`env LD_LIBRARY_PATH=/usr/local/buildsvc/lib /bin/bash ...` 段错误；移除该变量后 bash 正常。
- 证据等级：E3
- 关键日志/堆栈/输入：内核记录 `systemctl`、`run-build.sh`、`bash` segfault；随包 `libc.so.6` 为 glibc 2.27，系统 libc 为 glibc 2.23。
- 根因：`packaging/buildsvc.sh` 将随包库目录写入全局 `LD_LIBRARY_PATH`，该环境被 buildsvc 启动的所有子进程继承。
- 相关代码路径：`packaging/buildsvc.sh` 启动器；`src/agent.rs` 的 `run-build.sh`、`systemctl poweroff` 子进程。

## 3. 修复方案

- 最小修复点：删除启动器中的全局 `LD_LIBRARY_PATH` 导出。
- 代码逻辑改动：继续使用动态 loader 的 `--library-path "$LIB"` 加载 buildsvc，仅将随包库用于主进程加载，不污染子进程环境。
- 影响的使用场景：Linux 打包版 agent 的构建、终端、升级和关机命令。
- 不影响的使用场景：配置格式、WebSocket 协议、agent 身份 token 和 server 行为。

## 4. 修复执行计划

| 步骤 | 修改内容 | 验证方式 | 状态 |
|------|----------|----------|------|
| 1 | 删除启动器对 `LD_LIBRARY_PATH` 的全局注入 | `bash -n packaging/buildsvc.sh`、脚本审查 | 完成 |
| 2 | 验证 loader 仍使用随包库、子进程不继承该变量 | `bash -n`、`git diff --check`、`cargo test`；目标机部署后回归验证 | 本地完成，远端待部署 |

## 5. 风险摘要

| 项 | 结论 |
|----|------|
| 风险矩阵 | L2 |
| 命令权限 | C1（本地修改）；目标机部署需另行确认 |
| 高风险门禁 | 是（构建系统/动态链接运行时） |
| 破坏性操作 | 否 |
| 用户已有修改 | 否 |
| 用户确认 | 无；未执行远程部署 |
| 副作用/风险 | 依赖启动器的显式 `--library-path` 继续覆盖主进程依赖解析 |

## 6. 验证

- 验证环境：本地仓库；根因证据来自 `192.168.122.54` Ubuntu 16 目标机。
- 回归/相关验证：`bash -n packaging/buildsvc.sh`、`git diff --check`、`cargo test`（26 项通过）；目标机需重新打包部署后验证 bash、run-build.sh 和 systemctl。
- 结果：代码补丁和本地验证已完成，目标机尚未部署。
- 未执行验证项：未在目标机重启服务，未执行远程构建或关机。
- 残余风险：当前目标机仍运行旧启动脚本，问题在部署新包前不会消失。

## 7. 修复总结

- 最终结果：修复启动器对子进程的动态库环境污染。
- 计划偏差：无。
- 后续建议：重新打包并部署后重启 buildsvc，再验证 `run-build.sh` 和 systemctl 不再段错误。
