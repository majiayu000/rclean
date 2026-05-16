# rclean 架构决策 — Multi-AI Research artifact

- date: 2026-05-16
- topic: rclean 下一阶段架构决策（并发扫描 / MCP / graveyard / rule 引擎）
- 外部 AI 实际可用：gemini ✅ / grok ✅（重试后） / chatgpt ❌（两次 NETWORK_ERROR timeout）
- 本地证据：源码审计 agent（Explore）+ 竞品调研 agent（general-purpose）
- 模式标注：**partial multi-AI**（2 路外部 + 2 路本地）

## Phase 1：4 个决策点

1. 并发扫描库选型：jwalk vs ignore vs rayon+walkdir
2. MCP server 形态：高价值差异化 vs 过度押注
3. soft-delete graveyard：必要冗余 vs 重叠
4. rule 引擎抽象：trait object vs enum dispatch vs config-driven

## Phase 5：交叉验证矩阵

| # | 决策 | gemini | grok | 本地源码审计 | 本地竞品调研 | Tier | 结论 |
|---|------|--------|------|-------------|-------------|------|------|
| Q1 | 并发库 | **ignore(H)** crossbeam-deque + 自带 gitignore 状态机 | jwalk(M) 并行 DFS + 简单 | 推荐 rayon+ignore，预期 3-5× | 推荐 jwalk（dua-cli 同款）| 🔴 **Conflict** | 落点取决于是否启用 `.rcleanignore`（见下） |
| Q2 | MCP server | thin wrapper(M) | 是真窗口(H) | — | 是真窗口（mcpmarket 已有 3 个 bash 包装；Gemini 误删事件）| 🟢 **Strong consensus（方向）** | 都同意做，分歧在边界（核心 vs 独立 crate） |
| Q3 | graveyard | 不加(H) ActionPlan+trash 已重叠 | 加(M) AI 调用场景多保险 | — | 列入 Top 5 借鉴（trust model 闭环）| 🔴 **Conflict** | 取决于 AI agent 场景权重 |
| Q4 | rule 引擎 | TOML 配置 + 编译期硬编码 enum(H) | TOML + enum dispatch(H) | 拆 trait/enum dispatch | `.rcleanignore` 是 kondo 痛点 | 🟢 **Strong consensus** | 内置 enum dispatch + 用户 TOML/glob |

## Phase 5 关键观察

**Q1 冲突的根因**：jwalk 是纯并行 walker，ignore crate 额外自带 gitignore 状态机。如果 rclean 决定做 `.rcleanignore`（Q4 强共识），那 ignore crate 一举两得；如果 rclean 不打算让用户写 ignore 文件，jwalk 更轻量。**Q1 应该 follow Q4 决策结果**，不是独立选择。

**Q3 冲突的根因**：gemini 从"功能正交性 + 维护成本"角度否决；grok 从"AI agent 误删风险"角度肯定。本质是"押 AI 集成场景多重"还是"先打磨核心 trust model"。**Q3 应该 follow Q2 是否押 MCP**。

**Q2 / Q4 强共识**：MCP 集成是真窗口（4/4 信号肯定），rule 引擎走 enum dispatch + 用户 TOML（3/3 明确）。这两项可以直接落地。

## Phase 6 — Tiered action items 见正文方案对比
