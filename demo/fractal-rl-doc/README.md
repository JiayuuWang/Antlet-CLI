# Demo：分形技术文档生成 —— 《从经典强化学习到 Agentic RL》

本目录记录了一次 **Antlet 智能体集群** 的分形（fan-out / fan-in）协作 demo：
一个主编 Agent 并行调度 5 个章节作者子 Agent，各自独立撰写一节技术内容，
最后收割并拼接成一篇完整的中文技术文档。

## 场景设计

主题：**从经典强化学习到 Agentic RL**，按知识脉络拆成 5 节：

| 子 Agent | 章节 | persona（角色） |
|---|---|---|
| root.1 | 强化学习基础：MDP、价值函数与策略 | RL 资深作者 |
| root.2 | 经典算法：动态规划、蒙特卡洛与时序差分 | RL 资深作者 |
| root.3 | 深度强化学习：DQN、策略梯度与 Actor-Critic | 深度 RL 资深作者 |
| root.4 | 从 RLHF 到大模型对齐 | 大模型/RLHF 资深作者 |
| root.5 | Agentic RL：面向自主智能体的强化学习 | Agentic AI 资深作者 |

## 分形流程（fan-out / fan-in）

```
                    root（主编 Agent）
                         │
        ┌────────┬───────┼───────┬────────┐   ← fan-out：一次 spawn_agents 启动 5 个
      root.1   root.2  root.3  root.4   root.5    并行后台写作，互不干扰
        │        │       │       │        │
   section_1  section_2  ...   section_4 section_5  ← 各自用 write 落盘独立文件
        └────────┴───────┼───────┴────────┘   ← fan-in：stop_agent(wait=true) 逐个收割
                         │
                  final_document.md          ← reduce：读取拼接 + 目录
```

## 这个 demo 证明了什么

- **并行加速**：5 节内容由 5 个子 Agent **同时**撰写，墙钟时间约等于写 1 节（spawn 在同一步内完成，5 个子几乎同时返回 completed）。
- **角色专精**：每个子 Agent 有独立 `system_prompt`，被设定为对应主题的资深作者。
- **互不干扰**：每个子 Agent 拥有独立 session 文件与独立临时 profile 目录，写作过程完全隔离。
- **可控收割**：主编用 `stop_agent(wait=true)` 逐个优雅收割结果，确认完成状态。

## 文件清单

- `final_document.md` —— 最终拼接文档（约 20.7K 字符，5 节，含目录与 12 组 LaTeX 公式块）
- `section_1.md` ~ `section_5.md` —— 5 个子 Agent 各自产出的章节原文
- `run.log` —— **完整执行日志**（含彩色 ANSI 码），记录 spawn / write / completed / harvested 全过程

## 执行时间线（摘自 run.log）

```
14:25:24  DEMO START
14:25:xx  spawned root.1 ~ root.5（同一步内一次性 fan-out）
          各子 Agent 后台并行写作并 write 落盘
          root.1 completed → harvested
          root.2 completed → harvested
          root.4 completed
          root.3 completed → harvested
          root.4 harvested
          root.5 completed → harvested
          主编 read 5 节 + wc 统计字数
14:28:59  DEMO END
```

## 复现命令

```bash
export ANTLET_API_BASE=https://api.aipaibox.com/v1
export ANTLET_MODEL=claude-sonnet-4-6
export ANTLET_API_KEY=<your-key>
export TAVILY_API_KEY=<your-key>
export ANTLET_HOME=/tmp/antlet-demo/home

cargo run -- --workspace /tmp/antlet-demo/ws --max-steps 30 --task "<见下方编排 prompt>"
```

编排 prompt 的核心：固定 5 节大纲 → 一次 `spawn_agents`（数组长度 5，每个含 system_prompt + task，要求用 write 落盘）→ 逐个 `stop_agent(wait=true)` 收割 → read 拼接成 `final_document.md`。

## 已知现象（真实记录）

本次运行中，5 个子 Agent 的并行写作（fan-out）与逐个收割（fan-in）全部成功，
但主编在最后一步「读取拼接」时，于读完 5 节、统计完字数后提前结束（assistant 输出为空），
未自动生成 `final_document.md`。该 reduce 步骤是确定性的文件拼接，已由脚本补完。

这说明：**集群的并行编排与隔离机制工作稳健**；可优化点在于强化主编 reduce 阶段的收尾提示
（例如把"组装"拆成更明确的子步骤，或单独触发一次组装指令）。
