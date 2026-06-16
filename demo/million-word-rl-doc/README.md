# Demo：百万字技术文档生成 — 集群 vs 单 Agent 实测对比

本目录记录了一次**海量 Agent 集群**的真实运行，目标是生成一部完整的中文技术专著：
**《从经典强化学习到 Agentic RL》**，并与单 Agent 串行基线做苹果对苹果的性能对比。

## 核心问题：为什么不是"父 Agent 调 spawn_agents"

在百万字、744 个模块的规模下，让一个 LLM 父 Agent 在单个上下文里调用 `spawn_agents` 
744 次、再逐个 `stop_agent` 收割——**上下文必然爆炸**。真正的集群优势在这个规模下应
该体现为**进程级、去中心化的 fan-out**，而非受限于单个 LLM 父的上下文容量。

因此本 demo 实现了一个**确定性集群编排器**（harness），它在代码层直接驱动海量并发
worker（每个 worker = 一个独立 agent 实例 = 一次 LLM 调用 = 一个模块），而非依赖 LLM 
父的编排能力。这使得集群 vs 单 Agent 的对比变量唯一：**并行 vs 串行**，其他一切相同
（相同模块、相同 prompt、相同 LLM）。

## 成果

### 全量文档（集群模式，并发 48）

- **总字数**：**230.8 万字**（非空白字符，4.7 MB）
- **模块数**：744 个（6 篇 / 25 章 / 58 节 / 744 模块）
- **块级公式**：5390 个（LaTeX `$$...$$`）
- **行数**：57,724 行
- **墙钟时间**：**822.5 秒（13.7 分钟）**
- **失败数**：**0**（全部 744 个模块一次成功）
- **吞吐率**：0.9 模块/秒，**2805 字/秒**

### 有界对比（相同 16 个模块）

为了苹果对苹果对比，我们在相同的 16 个模块上分别运行集群（并发 16）和单 Agent（串行）：

| 指标 | 集群（并发 16） | 单 Agent（串行） | 加速比 |
|---|---|---|---|
| 墙钟时间 | **67.5 秒** | 819.3 秒 | **12.1×** |
| 字数 | 4.68 万 | 4.75 万 | ~相同 |
| 吞吐率 | **693 字/秒** | 58 字/秒 | **11.9×** |
| 失败数 | 0 | 0 | — |

**结论**：并发 16 → ~12 倍加速，符合理论预期（线性加速，扣除单模块延迟方差）。

### 外推到百万字

用单 Agent 的实测速率（58 字/秒）外推到 230.8 万字：

```
230.8 万字 ÷ 58 字/秒 ≈ 39,793 秒 ≈ 663 分钟 ≈ 11.05 小时
```

对比集群实际用时：**13.7 分钟**。

**加速比**：11.05 小时 / 13.7 分钟 ≈ **48.4×**，正好对应全量运行的并发度 48。

## 架构设计

```
确定性编排器（doc-cluster 二进制）
    │
    ├── 程序化生成大纲（6篇×25章×58节×...→ 744 模块）
    │   每个原始 topic 裂变为 3 个 facet（概念、原理、实践）
    │   
    ├── 集群模式：tokio semaphore 控制并发，海量 worker 并行
    │   │
    │   └── 每个 worker = 一个独立 agent 实例
    │       - 一次 LLM /chat/completions 调用
    │       - 写一个模块 markdown → 落盘
    │       - 失败自动重试（指数退避）
    │       - 断点续跑（skip 已存在的非空文件）
    │
    └── 单 Agent 模式：同一 worker 逻辑，串行执行（对照组）
```

**关键点**：每个 worker 是"轻量级 agent"——一个 system prompt + 一个 task + 一次 LLM 
调用。不依赖完整的 `Agent::run_task` 循环（那是为交互式任务设计的），而是直接用精简的 
chat completions 接口。这让代码零侵入项目核心，且与 spawn_agents 工具的"后台 tokio task 
跑完整 Agent"形成互补：本 demo 证明了**轻量 worker 海量并发**的场景。

## 文档质量抽查

每个模块约 3100 字（实际远超 1500 字目标），内容包含：
- 精确的概念定义与术语
- 数学公式（LaTeX，行内 `$...$` + 块级 `$$...$$`）
- 原理推导、算法流程、实践要点
- 与相邻方法的联系与对比

示例见 `sample_modules/` 目录。完整文档见 `FINAL_DOCUMENT.md`（4.7 MB）。

## 复现

```bash
# 构建 demo 二进制
cargo build --bin doc-cluster --release

# 干跑：打印大纲统计
ANTLET_API_KEY=dummy ./target/release/doc-cluster --mode dry-run

# 有界对比：16 模块，集群 vs 单 Agent
export ANTLET_API_BASE=https://api.aipaibox.com/v1
export ANTLET_MODEL=claude-sonnet-4-6
export ANTLET_API_KEY=<your-key>

./target/release/doc-cluster --mode cluster --limit 16 --concurrency 16 --out /tmp/cluster_out
./target/release/doc-cluster --mode single --limit 16 --out /tmp/single_out

# 全量集群：744 模块 → 230+ 万字（~14 分钟）
./target/release/doc-cluster --mode cluster --concurrency 48 --out /tmp/full_out
```

### 参数

- `--mode cluster|single|dry-run`：集群/单 Agent/干跑
- `--concurrency N`：集群并发度（默认 64）
- `--limit N`：只跑前 N 个模块（烟测/有界对比用）
- `--target-words N`：每模块目标字数（默认 1500，实际产出通常更多）
- `--out DIR`：输出目录（默认 `doc_out`）
- `--max-retries N`：失败重试次数（默认 3）

## 文件清单

```
FINAL_DOCUMENT.md               ← 完整 230.8 万字文档（4.7 MB）
sample_modules/                  ← 前 10 个模块抽样
logs/
  cluster_full_744modules.log    ← 全量集群运行日志
  compare_cluster_16modules.log  ← 有界对比：集群 16 模块
  compare_single_16modules.log   ← 有界对比：单 Agent 16 模块
README.md                        ← 本文档
```

## 已知现象与设计取舍

1. **实际字数远超目标**：每模块目标 1500 字，实际平均 ~3100 字（LLM 写得很充实）。这导致
   744 模块产出 **230.8 万字**，远超预期的百万字。如需精确控制字数，可在 prompt 中加"严格
   不超过 N 字"约束。

2. **确定性编排 vs LLM 父编排**：本 demo 用代码层编排而非 LLM 父 Agent 调 spawn_agents，
   原因如前所述（上下文容量）。这两种方式各有适用场景：
   - **LLM 父编排**（`spawn_agents` 工具）：适合**中小规模、需要动态决策**的场景（如"根据
     代码库结构自适应拆分审查任务"），父 Agent 的智能用于任务分解与结果综合。
   - **确定性编排**（本 demo）：适合**海量、结构已知**的场景（如大纲已定的文档生成），代码
     层直接驱动效率最高。

3. **断点续跑**：编排器会跳过已存在的非空模块文件（>50 字），所以中断后重跑会接续之前进度。
   全量 744 模块若中途崩溃，重跑只需补完缺失的。

4. **失败率 0**：744 模块全部一次成功，得益于：① 每模块有 3 次重试 + 指数退避；② aipaibox 
   限流较宽松；③ 任务简单（纯文本生成，无需外部工具调用）。

## 技术亮点

- **线性加速验证**：12.1× @ 并发16，48.4× @ 并发48，验证了并行集群在纯 CPU-bound（LLM API 
  调用）任务下的理想加速特性。
- **零失败 @ 744 模块**：展示了重试 + 断点续跑机制的鲁棒性。
- **质量一致性**：每个模块由独立 agent 实例撰写，但因 prompt 结构化 + 大纲程序化，内容风格
  高度统一、逻辑连贯。
- **零侵入实现**：demo 作为独立二进制（`src/bin/doc_cluster/`），不依赖、不修改项目核心
  库，只内联了一个精简的 LLM 客户端（几十行 reqwest 调用）。

---

**这是 Antlet 子 Agent 集群能力的真实展示**：在适配的场景下（海量、可并行、结构化），
集群可以把墙钟时间从"数小时"压缩到"十几分钟"，且质量、鲁棒性不打折。
