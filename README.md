# Antlet 开发计划

> 一个 nano 版本的 Agent 调度系统，用 Rust 实现多 Agent 调度、Memory 和定时任务核心功能。

---

## 项目目标

- **代码量**：尽量少
- **核心功能**：多 Agent 调度 + Memory 管理 + 长期待机+定时任务触发
- **依赖**：只使用必要的 3-4 个核心 crate
- **定位**：引导开发者入门，具有教育意义

---

- https://github.com/RightNow-AI/openfang
- https://opencode.ai/docs/zh-cn
- https://code.claude.com/docs/zh-CN/overview
- https://github.com/openclaw/openclaw
- https://github.com/zeroclaw-labs/zeroclaw
- https://jiayuuwang.github.io/%E8%84%9A%E6%89%8B%E6%9E%B6/%E4%BB%A3%E7%A0%81%E7%94%9F%E6%88%90%E6%99%BA%E8%83%BD%E4%BD%93/the-generalization-of-agent-scaffolding-cn/

## 项目结构（大模型生成，仅供参考）

```
antlet/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # 库入口，导出核心类型
│   ├── agent.rs            # Agent trait 定义
│   ├── context.rs          # 执行上下文（含 Memory 访问）
│   ├── memory.rs           # Memory 实现（短期+长期）
│   ├── scheduler.rs        # 调度器实现
│   ├── task.rs             # 任务定义
│   ├── cron.rs             # 定时任务封装
│   └── examples/           # 示例代码
│       ├── echo_agent.rs   # 最简单的 Agent
│       ├── multi_agent.rs  # 多 Agent 调度示例
│       └── cron_demo.rs    # 定时任务示例
└── tests/
    └── integration.rs      # 集成测试
```

---

## 依赖选型（大模型生成，仅供参考）

```toml
[package]
name = "antlet"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["rt", "rt-multi-thread", "time", "macros"] }
async-trait = "0.1"
sled = "0.34"
tokio-cron-scheduler = "0.11"
anyhow = "1"
tracing = "0.1"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
```

---

## 模块详细设计（大模型生成，仅供参考）

### 1. Agent Trait (`agent.rs`)

```rust
use async_trait::async_trait;

#[async_trait]
pub trait Agent: Send + Sync {
    /// Agent 名称
    fn name(&self) -> &str;
    
    /// 执行 Agent 逻辑
    async fn run(&self, ctx: &Context, input: &str) -> anyhow::Result<String>;
}
```

### 2. Context (`context.rs`)

```rust
pub struct Context {
    memory: Memory,
    // 可扩展：会话 ID、日志等
}

impl Context {
    pub fn new() -> Self;
    pub fn memory(&self) -> &Memory;
}
```

### 3. Memory (`memory.rs`)

```rust
pub struct Memory {
    short_term: VecDeque<Message>,  // 滑动窗口
    long_term: sled::Db,             // 持久化 KV
}

pub struct Message {
    pub role: String,   // "user" / "assistant"
    pub content: String,
    pub timestamp: u64,
}

impl Memory {
    pub fn new(db_path: Option<&str>) -> Self;
    
    /// 短期记忆：添加消息，自动维护窗口大小
    pub fn push(&mut self, role: &str, content: &str);
    
    /// 获取最近 N 条消息
    pub fn recent(&self, n: usize) -> Vec<&Message>;
    
    /// 长期记忆：存储/读取任意 KV
    pub fn set(&self, key: &str, value: &[u8]) -> anyhow::Result<()>;
    pub fn get(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>>;
}
```

### 4. Scheduler (`scheduler.rs`)

```rust
pub struct Scheduler {
    agents: Vec<Box<dyn Agent>>,
    round_robin: AtomicUsize,
    task_tx: mpsc::Sender<Task>,
    task_rx: mpsc::Receiver<Task>,
}

pub struct Task {
    pub agent_name: String,
    pub input: String,
    pub response_tx: oneshot::Sender<anyhow::Result<String>>,
}

impl Scheduler {
    pub fn new() -> Self;
    
    /// 注册 Agent
    pub fn register(&mut self, agent: Box<dyn Agent>);
    
    /// 提交任务（异步）
    pub async fn submit(&self, agent_name: &str, input: &str) -> anyhow::Result<String>;
    
    /// 启动调度器（轮询任务队列）
    pub async fn start(&self);
}
```

### 5. Cron 任务封装 (`cron.rs`)

```rust
use tokio_cron_scheduler::JobScheduler;

pub struct CronScheduler {
    inner: JobScheduler,
}

impl CronScheduler {
    pub fn new() -> Self;
    
    /// 添加定时任务
    pub fn add_job(
        &self,
        cron_expr: &str,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> anyhow::Result<()>;
    
    /// 添加触发 Agent 的定时任务
    pub fn schedule_agent(
        &self,
        cron_expr: &str,
        scheduler: &Scheduler,
        agent_name: &str,
        input: String,
    ) -> anyhow::Result<()>;
    
    /// 启动调度器
    pub async fn start(&self) -> anyhow::Result<()>;
}
```

---

## 分阶段开发计划（大模型生成，仅供参考）

### Phase 1: 基础抽象 (Day 1-2) | ~300 行

- [ ] 创建项目结构
- [ ] 定义 `Agent` trait
- [ ] 定义 `Context` 结构
- [ ] 实现 `EchoAgent` 示例
- [ ] 单元测试

### Phase 2: 调度器 (Day 2-3) | ~400 行

- [ ] 实现 `Scheduler` 结构
- [ ] 实现 Agent 注册
- [ ] 实现轮询调度算法
- [ ] 实现任务提交和异步响应
- [ ] 多 Agent 调度示例

### Phase 3: Memory (Day 3-4) | ~350 行

- [ ] 集成 `sled`
- [ ] 实现短期记忆（滑动窗口）
- [ ] 实现长期记忆 KV 存储
- [ ] 在 Context 中集成 Memory
- [ ] 带记忆的 Agent 示例

### Phase 4: 定时任务 (Day 4-5) | ~250 行

- [ ] 集成 `tokio-cron-scheduler`
- [ ] 封装 `CronScheduler`
- [ ] 实现定时触发 Agent 的便捷方法
- [ ] 定时任务示例

### Phase 5: 完善与测试 (Day 5-6) | ~200 行

- [ ] 编写集成测试
- [ ] 错误处理完善（anyhow 细化）
- [ ] 添加 tracing 日志
- [ ] 编写 README 和文档注释
- [ ] 性能基准测试（可选）

---

## 快速开始示例（大模型生成，仅供参考）

```rust
use antlet::{Agent, Context, Scheduler, CronScheduler};

#[derive(Default)]
struct HelloAgent;

#[async_trait]
impl Agent for HelloAgent {
    fn name(&self) -> &str { "hello" }
    
    async fn run(&self, ctx: &Context, input: &str) -> anyhow::Result<String> {
        let greeting = format!("Hello, {}!", input);
        ctx.memory().push("assistant", &greeting);
        Ok(greeting)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut scheduler = Scheduler::new();
    scheduler.register(Box::new(HelloAgent));
    
    let scheduler_handle = scheduler.start();
    
    let result = scheduler.submit("hello", "Antlet").await?;
    println!("{}", result);
    
    let cron = CronScheduler::new();
    cron.schedule_agent("0 9 * * * *", &scheduler, "hello", "morning".to_string())?;
    cron.start().await?;
    
    Ok(())
}
```

---

## 进度跟踪（大模型生成，仅供参考）

| Phase | 状态 | 预估行数 |
|-------|------|----------|
| Phase 1 | ⬜ 未开始 | ~300 |
| Phase 2 | ⬜ 未开始 | ~400 |
| Phase 3 | ⬜ 未开始 | ~350 |
| Phase 4 | ⬜ 未开始 | ~250 |
| Phase 5 | ⬜ 未开始 | ~200 |
| **总计** | - | **~1500** |

---

*Happy Coding! 🐜*
```
---

## 当前可运行版本（Rust Mini Coding Agent）

### 能力

- OpenAI 兼容协议 LLM 调用（默认 `https://api.minimaxi.com/v1`）
- Agent 执行循环（LLM -> 工具调用 -> 工具结果 -> LLM）
- 工具：`read_file` / `write_file` / `apply_patch` / `repo_search` / `bash` / `web_search(Tavily)`
- JSONL 会话持久化（`.antlet/sessions/<session>.jsonl`）
- CLI：交互模式与 `--task` 单次任务模式

### 环境变量

```bash
export ANTLET_API_KEY="your_key"
export ANTLET_API_BASE="https://api.minimaxi.com/v1"   # 可选
export ANTLET_MODEL="MiniMax-M2.5"                      # 可选
export TAVILY_API_KEY="your_tavily_key"                 # 使用 web_search 时需要
```

### 运行

```bash
# 交互模式
cargo run -- --workspace . --session demo

# 单次任务模式
cargo run -- --workspace . --task "Read README.md and summarize architecture"
```

### 交互命令

- `/history` 查看消息条数
- `/clear` 清空历史（保留 system prompt）
- `/exit` 退出
