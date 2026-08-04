# Gentoo 包动态加载器权限修复记录

> 文档元数据
> - 文件编号：19
> - 文档类型：fix
> - 文件路径：docs/dev/19-fix-gentoo-loader-permission.md
> - 完成日期：2026-08-04
> - 需求级别：L2

## 1. 问题定义

- 问题现象：本机 `buildsvc.service` 启动后以 `status=1` 退出，并进入 `auto-restart`。
- 预期行为：Gentoo 安装的服务可以通过启动脚本正常执行动态加载器并启动 buildsvc。
- 影响范围：Gentoo 打包产物的服务启动；Deb、RPM 安装流程不受影响。
- 不包含：不修改 systemd 重启策略、运行时配置和已有用户数据。

## 2. 证据与根因

- 复现方式：执行 `/usr/local/buildsvc/scripts/buildsvc.sh`。
- 证据等级：E2
- 关键日志/堆栈/输入：脚本输出 `missing dynamic loader in: /usr/local/buildsvc/lib`；已安装 `/usr/local/buildsvc/lib/ld-linux-x86-64.so.2` 权限为 `0644`。
- 根因：Gentoo ebuild 使用 `doins` 安装所有运行库，导致动态加载器失去执行权限；启动脚本按 `-x` 检查时拒绝使用它。
- 相关代码路径：`packaging/package.sh` 的 Gentoo `src_install`；`packaging/buildsvc.sh` 的动态加载器检查。

## 3. 修复方案

- 最小修复点：Gentoo `src_install` 完成库文件安装后，仅将 `ld-linux*.so*` 设置为 `0755`。
- 代码逻辑改动：遍历 `${ED}/usr/local/buildsvc/lib/ld-linux*.so*`，使用 `fperms 0755` 修正目标路径权限。
- 影响的使用场景：重新构建并安装 Gentoo 包后，启动脚本可以找到可执行动态加载器。
- 不影响的使用场景：普通共享库仍按原有 `doins` 安装；Deb、RPM 使用现有 `install -Dm755` 流程。

## 4. 修复执行计划

| 步骤 | 修改内容 | 验证方式 | 状态 |
|------|----------|----------|------|
| 1 | 在 Gentoo ebuild 安装阶段恢复动态加载器执行权限 | `bash -n packaging/package.sh`、代码审查 | 完成 |
| 2 | 检查生成 ebuild 的语法和变更范围 | `bash -n packaging/package.sh`、`git diff --check` | 完成 |
| 3 | 运行 Rust 回归测试，确认无代码行为回归 | `cargo test` | 完成 |

## 5. 风险摘要

| 项 | 结论 |
|----|------|
| 风险矩阵 | L2 |
| 命令权限 | C0 / C1 |
| 高风险门禁 | 是（构建/发布包内容变更） |
| 破坏性操作 | 否 |
| 用户已有修改 | 否；保留 `Cargo.lock`、UI 和已有本地文档变更 |
| 用户确认 | 无 |
| 副作用/风险 | 仅改变 Gentoo 包内动态加载器权限；未在本机重新安装或重启服务 |

## 6. 验证

- 验证环境：本地 Linux 工作区。
- 回归/相关验证：`bash -n packaging/package.sh`、`cargo test`、`git diff --check`；人工确认 ebuild 使用 `${ED}` 定位安装目标并仅匹配动态加载器。
- 结果：通过；Rust 测试 26 项全部通过。
- 未执行验证项：未重新构建 Gentoo 包，未执行安装包和服务重启验证。
- 残余风险：最终权限仍需在重新生成并安装 Gentoo 包后，通过 `stat` 和服务启动确认。

## 7. 修复总结

- 最终结果：Gentoo 打包流程会将动态加载器安装为可执行文件，解决启动脚本因 `-x` 检查失败导致的重启循环。
- 计划偏差：无。
- 后续建议：重新打包安装后确认 `/usr/local/buildsvc/lib/ld-linux*.so*` 为 `0755`，再启动服务。
