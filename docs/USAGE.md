# Antlet Agent 使用说明

## 1. 安装与准备

### 前置
- Rust 1.94+
- 可访问 OpenAI 兼容接口（例如 MiniMax OpenAI 风格网关）

### 拉起项目
```bash
cargo build
```

## 2. 配置

必填环境变量：
```bash
export ANTLET_API_KEY="your_api_key"
```

可选环境变量：
```bash
export ANTLET_API_BASE="https://api.minimaxi.com/v1"
export ANTLET_MODEL="MiniMax-M2.5"
export ANTLET_HOME="$HOME/.antlet"             # 运行数据目录
export ANTLET_PROFILE_DIR="$HOME/.antlet/profile" # 系统提示词目录（可选）
export ANTLET_PROFILE_RESET="0"                # 可选：1/true 时用内置模板覆盖 profile 文件
export TAVILY_API_KEY="your_tavily_key"        # 仅 web_search 工具需要
```

## 3. 启动方式

### 交互模式
```bash
cargo run -- --workspace /path/to/your/project --session demo
```

### 单次任务模式
```bash
cargo run -- --workspace /path/to/your/project --task "修复 src/main.rs 的编译错误"
```

说明：
- 可在任意工作目录启动 `antlet-agent`。
- 通过 `--workspace` 指定真正要操作的工程目录。
- 会话记录保存在 `~/.antlet/sessions/<session>.jsonl`。

## 4. 系统提示词配置（仿 openclaw）

每次会话启动时，Agent 会自动读取以下 markdown 文件并注入 system prompt：

- `persona.md`
- `self_knowledge.md`
- `principles.md`
- `workflow.md`

默认目录：`~/.antlet/profile`（可由 `ANTLET_PROFILE_DIR` 覆盖）。

行为细节：
- 第一次启动时若文件不存在，会自动创建模板。
- 之后你可以直接编辑这些 `.md`。
- 每次会话开始时都会重新读取最新内容并覆盖当前会话的 system message。
- 若要恢复内置模板，可临时设置 `ANTLET_PROFILE_RESET=1` 后再启动一次。

## 5. 控制台输出说明

启动时会打印：
- Antlet Logo
- 关键配置：workspace、session、model、api_base、profile_dir、profile_files、max_steps、tools

运行中会按颜色区分：
- `user>`：用户消息
- `assistant>`：模型回复
- `tool>`：工具调用
- `tool.args>`：工具参数
- `tool.ok>`：工具成功输出
- `tool.err>`：工具失败输出

## 6. 内置工具

- `read_file`: 读取文件（带行号）
- `apply_patch`: 编辑文件（基于文本替换）
- `bash`: 执行命令（默认开放，带超时）
- `web_search`: Tavily 网页搜索

## 7. 交互命令

- `/history` 查看当前消息数量
- `/clear` 清空历史（保留 system prompt）
- `/exit` 退出

## 8. 常见问题

### Q: web_search 报错 missing TAVILY_API_KEY
A: 设置 `TAVILY_API_KEY` 后重试。

### Q: 为什么启动目录变了还能继续用同一个会话？
A: 会话统一落在 `ANTLET_HOME`（默认 `~/.antlet`），与当前启动目录解耦。
