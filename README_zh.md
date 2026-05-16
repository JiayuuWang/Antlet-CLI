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
export ANTLET_MODEL="MiniMax-M2.7"                      # 可选
export TAVILY_API_KEY="your_tavily_key"                  # 可选，用于 web_search 工具
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
| `TAVILY_API_KEY` | 否 | - | 使用 web_search 工具时需要 |

---

## 系统提示词

启动时 Antlet 会从 `~/.antlet/profile/`（或 `ANTLET_PROFILE_DIR`）读取 markdown 文件来构建系统提示词：

- `persona.md` - Agent 角色设定
- `self_knowledge.md` - Agent 自我认知
- `principles.md` - 行为准则
- `workflow.md` - 工作流程描述

首次运行时会自动创建模板文件，可直接编辑这些文件来定制 Agent 行为。

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

会话保存在 `~/.antlet/sessions/<session>.jsonl`，会话跨重启持久化，与工作目录解耦。

---

## 命令行参数

```bash
--workspace PATH    工作目录（默认: .）
--session NAME       会话名称（默认: default）
--task TEXT          单次任务模式
--max-steps N        最大循环次数（默认: 20）
--api-base URL       覆盖 API 端点
--model NAME         覆盖模型名称
```

## 交互命令

- `/history` - 查看消息条数
- `/clear` - 清空历史（保留 system prompt）
- `/exit` - 退出

---

## 架构

```
src/
├── main.rs          # 入口，CLI 解析，交互循环
├── agent.rs         # Agent 结构体，run_task 循环
├── llm.rs           # LlmClient，OpenAI 兼容 /chat/completions 调用
├── schema.rs        # Message, ToolCall, FunctionCall 类型定义
├── tools/
│   ├── mod.rs       # ToolRegistry, Tool trait
│   ├── read_file.rs
│   ├── apply_patch.rs
│   ├── bash.rs
│   └── web_search.rs
├── config.rs        # AppConfig，环境变量加载
├── profile.rs       # 从 .md 文件构建系统提示词
├── session_store.rs # JSONL 会话持久化
└── ui.rs            # 彩色终端输出
```

核心循环在 `Agent::run_task()`：发送消息 + 工具描述 → LLM → 若有工具调用则执行后重复 → 否则返回文本。

---

## License

MIT