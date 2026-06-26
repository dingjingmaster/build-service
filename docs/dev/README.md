# 需求与任务索引

> 记录 L1+ 新增任务、问题、调研、计划、总结或评审文档。本索引是本地上下文，默认不要求提交。新需求/问题优先读取本索引，再按相关性最多展开 3-5 个 Summary、task 或 fix 文档，避免加载完整历史上下文。

## 编号规则

- 文件编号使用从 `1` 开始递增的正整数，不要求固定位数。
- 编号全局只在 `docs/dev/` 下递增，不按类型分别编号。
- 新增任务、问题修复、调研、计划、总结、评审或需求变更文档前，先检查本索引和 `docs/dev/` 现有文件名，取最大编号 + 1。
- 同一需求的多份文档使用同一编号，例如 `2-research-xxx.md`、`2-plan-xxx.md`、`2-summary-xxx.md`。
- 编号一旦分配不得复用；取消、废弃、拆分、合并也要在索引中保留记录并标注状态。
- 文件命名格式：`N-[type]-[slug].md`，其中 `type` 可取 `summary`、`task`、`fix`、`research`、`plan`、`review`。

## 索引

| ID | 日期 | 级别 | 类型 | 文档 | 状态 | 摘要 |
|----|------|------|------|------|------|------|
| 1 | 2026-06-26 | L3 | research | [1-research-buildsvc-mvp.md](1-research-buildsvc-mvp.md) | 已完成 | Rust 自研轻量构建分发系统 MVP 调研。 |
| 1 | 2026-06-26 | L3 | plan | [1-plan-buildsvc-mvp.md](1-plan-buildsvc-mvp.md) | 已完成 | Rust 自研轻量构建分发系统 MVP 实施计划。 |
| 1 | 2026-06-26 | L3 | summary | [1-summary-buildsvc-mvp.md](1-summary-buildsvc-mvp.md) | 已完成 | Rust 自研轻量构建分发系统 MVP 实现、验证和残余风险总结。 |
