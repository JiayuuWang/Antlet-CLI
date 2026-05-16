# Antlet Agent

> 一个用 Rust 构建的 nano 编程 Agent，支持多 Agent 调度、Memory 管理和定时任务触发。

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
export ANTLET_API_BASE="https://api.minimaxi.com/v1"   # 可选，默认为 MiniMax
export ANTLET_MODEL="MiniMax-M2.5"                      # 可选
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

## 配置项

| 变量 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `ANTLET_API_KEY` | 是 | - | LLM 的 API Key |
| `ANTLET_API_BASE` | 否 | `https://api.minimaxi.com/v1` | OpenAI 兼容端点 |
| `ANTLET_MODEL` | 否 | `MiniMax-M2.5` | 模型名称 |
| `ANTLET_HOME` | 否 | `~/.antlet` | 数据目录 |
| `ANTLET_PROFILE_DIR` | 否 | `~/.antlet/profile` | 系统提示词模板目录 |
| `ANTLET_PROFILE_RESET` | 否 | `0` | 设为 `1` 可重置提示词模板 |
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
├── agent.rs         # Agent 结构体，run_task 循环
├── llm.rs           # LlmClient，OpenAI 兼容 /chat/completions 调用
├── schema.rs        # Message, ToolCall, FunctionCall 类型定义
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
│   └── search.rs    # Tavily 网页搜索
├── config.rs        # AppConfig，环境变量加载
├── profile.rs       # 从 .md 文件构建系统提示词
├── session_store.rs # JSONL 会话持久化
└── ui.rs            # 彩色终端输出
```

核心循环在 `Agent::run_task()`：发送消息 + 工具描述 → LLM → 若有工具调用则执行后重复 → 否则返回文本。

---

## License

MIT