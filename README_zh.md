# Antlet Agent

> 一个用 Rust 构建的 nano 编程 Agent，核心特色是可递归的 **子 Agent 集群**，并支持 Memory 管理和定时任务触发。

[English](./README.md) | [中文](./README_zh.md)

---

## 快速开始

### 1. 构建

```bash
cargo build
```

### 2. 配置

```bash
export ANTLET_API_KEY="your_api_key"
export ANTLET_API_BASE="your_base_url"   # 可选
export ANTLET_MODEL="model_name"                      # 可选
export TAVILY_API_KEY="your_tavily_key"                  # 可选，用于 search 工具
```

### 3. 运行

```bash
# 交互模式
cargo run -- --workspace /path/to/your/project --session demo

# 单次任务模式
cargo run -- --workspace /path/to/your/project --task "修复 src/main.rs 的编译错误"
```

---

## 子 Agent 集群

Antlet 的核心特色：任何一个 Agent 都能**派生一组子 Agent 集群**并行执行任务，
随后**收割**它们的结果——而且子 Agent 自己也能继续派生子 Agent，**深度不设上限**。
这让单个 Agent 升级为编排者，把工作以树状结构分发给一群专精的 worker。

### 为什么强大

- **并行加速** —— 一次调用启动 N 个子 Agent，它们在后台并发运行，N 个任务的墙钟时间约等于 1 个任务。
- **角色专精** —— 每个子 Agent 拥有独立的 `system_prompt`（人格），同一个问题可以同时由多个专家视角攻克。
- **完全隔离** —— 每个 Agent（父或子）都有独立的 session 文件和独立的临时 profile 目录，memory 与人格写入互不冲突，子 Agent 绝不污染父的上下文。
- **无限递归** —— 子 Agent 同样持有 spawn/stop 工具，形成一棵树（`root.1`、`root.1.2` …）。软性全局上限 `ANTLET_MAX_LIVE_SUBAGENTS`（默认 `64`，设 `0` 解除）作为安全阀。
- **可控收割** —— 父 Agent 可优雅收割子的完整结果（`wait=true`），或强制中止（`wait=false`）；停止会级联到所有后代并释放资源。

### 工作原理（fan-out / fan-in）

```
                    root agent（父）
                       │
      ┌────────┬───────┼───────┬────────┐   ← fan-out：一次 spawn_agents 调用
   root.1   root.2  root.3  root.4   root.5    并行启动 N 个子 Agent
      │        │       │       │        │
   (工作)    (工作)  (工作)  (工作)   (工作)   ← 各自独立运行、互相隔离
      └────────┴───────┼───────┴────────┘   ← fan-in：stop_agent(wait=true) 逐个收割
                       │
                  父 Agent 汇总结果
```

### 两个集群工具

| 工具 | 用途 |
|------|------|
| `spawn_agents` | 启动一个或多个子 Agent。`agents` 数组大小 = 子 Agent 数量。每个元素含必需的 `system_prompt`，以及可选的 `task` 和各 `.md` 初始值（`identities`、`self_knowledge`、`behavior`）。非阻塞——立即返回子 Agent 的 id。 |
| `stop_agent` | 查看与停止子 Agent。`list: true` 报告所有子状态；`agent_id` + `wait: true` 收割已完成子的完整结果；`wait: false` 强制中止；`all: true` 作用于全部子。会级联到后代。 |

### `spawn_agents` 参数

```jsonc
{
  "agents": [
    {
      "system_prompt": "你是一名安全审查员。",   // 必需：子 Agent 人格
      "task": "审查 src/ 的注入风险。",            // 可选：具体任务
      "identities": "...",                          // 可选：初始化 identities.md
      "self_knowledge": "...",                      // 可选：初始化 self_knowledge.md
      "behavior": "..."                             // 可选：初始化 behavior.md
    }
    // …每个子 Agent 一个元素
  ]
}
```

### 示例 prompt

```text
派生 3 个子 Agent 并行审查 src/main.rs：
  1. 安全审查员，2. 性能审查员，3. 可维护性审查员。
完成后用 stop_agent（wait=true）逐个收割结果，汇总成一份报告。
```

> 一个完整的端到端 demo（由 5 个子 Agent 集群协作撰写的 5 节技术文档）及完整执行日志见 [`demo/fractal-rl-doc/`](./demo/fractal-rl-doc/)。

---

## 配置项

| 变量 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `ANTLET_API_KEY` | 是 | - | LLM 的 API Key |
| `ANTLET_API_BASE` | 否 | `https://api.minimaxi.com/v1` | OpenAI 兼容端点 |
| `ANTLET_MODEL` | 否 | `MiniMax-M2.5` | 模型名称 |
| `ANTLET_HOME` | 否 | `~/.antlet` | 数据目录 |
| `ANTLET_PROFILE_DIR` | 否 | `~/.antlet/profile` | 系统提示词模板目录 |
| `ANTLET_PROFILE_RESET` | 否 | `0` | 设为 `1` 可重置提示词模板 |
| `ANTLET_MAX_LIVE_SUBAGENTS` | 否 | `64` | 同时活跃子 Agent 的软性上限（`0` = 不限） |
| `TAVILY_API_KEY` | 否 | - | 使用 search 工具时需要 |

---

## 定时任务

支持 cron 表达式和一次性时间戳两种调度方式。

```bash
# 添加一个 cron 任务（每10分钟执行）
cargo run -- --schedule "*/10 * * * *" --schedule-name "健康检查" --workspace . --task "检查系统状态"

# 添加一个一次性任务（unix 时间戳）
cargo run -- --schedule "1747500000" --schedule-name "部署" --workspace . --task "执行部署脚本"
```

**交互模式下**，调度器在后台运行，自动触发任务。使用 `/schedule` 查看所有定时任务。

### 调度格式

- **Cron**: `分 时 日 月 周` — 例如 `0 9 * * *`（每天9点）, `*/10 * * * *`（每10分钟）
- **一次性**: unix 时间戳（秒）— 例如 `1747500000`

### 交互命令

- `/history` - 查看消息条数
- `/clear` - 清空历史（保留 system prompt）
- `/schedule` - 查看定时任务列表
- `/exit` - 退出

---

## 内置工具

| 工具 | 说明 |
|------|------|
| `read` | 读取文件内容（带行号） |
| `write` | 写入文件或文本替换编辑 |
| `grep` | 在文件中搜索正则表达式 |
| `glob` | 按名称模式查找文件 |
| `ls` | 查看目录内容 |
| `bash` | 执行 Shell 命令 |
| `search` | 通过 Tavily 进行网页搜索 |
| `spawn_agents` | 派生并行运行的子 Agent 集群（见 [子 Agent 集群](#子-agent-集群)） |
| `stop_agent` | 查看、收割或停止子 Agent |

---

## 会话

会话保存在 `~/.antlet/sessions/<session>.jsonl`，定时任务保存在 `~/.antlet/scheduled_tasks.json`。会话跨重启持久化，与工作目录解耦。

---

## 命令行参数

```bash
--workspace PATH    工作目录（默认: .）
--session NAME      会话名称（默认: default）
--task TEXT         单次任务模式
--max-steps N       最大循环次数（默认: 20）
--api-base URL      覆盖 API 端点
--model NAME        覆盖模型名称
--schedule CRON     添加定时任务（cron 表达式）
--schedule-name     定时任务名称
```

---

## 系统提示词

启动时 Antlet 会从 `~/.antlet/profile/`（或 `ANTLET_PROFILE_DIR`）读取 markdown 文件来构建系统提示词：

- `persona.md` - Agent 角色设定与价值观
- `capabilities.md` - 工具描述与使用方法
- `self_knowledge.md` - Agent 能/不能做什么
- `behavior.md` - 执行规则与回复风格

首次运行时会自动创建模板文件，可直接编辑这些文件来定制 Agent 行为。

---

## 架构

```
src/
├── main.rs          # 入口，CLI 解析，交互循环
├── agent.rs         # Agent 结构体，run_task 循环，协作式取消
├── llm.rs           # LlmClient，OpenAI 兼容 /chat/completions 调用
├── schema.rs        # Message, ToolCall, FunctionCall 类型定义
├── subagent.rs      # AgentFactory + SubAgentManager：派生/停止，集群树
├── scheduler.rs     # 定时任务调度器，支持 cron 和一次性任务
├── schedule_store.rs # 定时任务 JSON 持久化
├── tools/
│   ├── mod.rs       # ToolRegistry, Tool trait
│   ├── read.rs      # 读取文件（带行号）
│   ├── write.rs     # 写入文件或文本替换
│   ├── grep.rs      # 正则搜索文件
│   ├── glob.rs      # 按模式找文件
│   ├── ls.rs        # 列出目录
│   ├── bash.rs      # 执行 Shell 命令
│   ├── search.rs    # Tavily 网页搜索
│   ├── profile_write.rs # 写入 profile .md 文件
│   ├── spawn.rs     # spawn_agents 工具（启动子 Agent 集群）
│   └── stop.rs      # stop_agent 工具（查看/收割/中止子 Agent）
├── config.rs        # AppConfig，环境变量加载
├── profile.rs       # 从 .md 文件构建系统提示词
├── session_store.rs # JSONL 会话持久化
└── ui.rs            # 彩色终端输出
```

核心循环在 `Agent::run_task()`：发送消息 + 工具描述 → LLM → 若有工具调用则执行后重复 → 否则返回文本。

子 Agent 编排逻辑位于 `subagent.rs`：不可变的 `AgentFactory`（整棵树共享）以与 root
完全相同的方式构造子 Agent，而每个 `SubAgentManager` 节点管理自己的直接子节点——
由此实现递归派生、session/profile 隔离以及级联停止。

---

## License

MIT