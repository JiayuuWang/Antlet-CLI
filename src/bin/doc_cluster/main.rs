//! Million-character technical-document generation demo.
//!
//! Compares two strategies on the SAME task (write the RL → Agentic RL
//! curriculum, module by module):
//!
//!   --mode cluster : a deterministic orchestrator fans the modules out across
//!                    many concurrent worker agents (bounded by --concurrency).
//!   --mode single  : one agent writes every module sequentially.
//!
//! Why a deterministic orchestrator instead of an LLM "parent" calling
//! spawn_agents? At ~1350 modules an LLM parent would blow its own context
//! managing 1350 spawn/harvest calls. The real strength of an agent cluster at
//! this scale is process-level, decentralized fan-out — which is exactly what
//! this harness drives, while each leaf module is still written by an
//! independent agent instance (its own prompt, its own LLM call, its own file).
//!
//! Each worker == one agent instance == one module == one output file. This
//! keeps the cluster-vs-single comparison apples-to-apples: identical modules,
//! identical prompts, the only variable is parallel vs sequential.

mod outline;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Semaphore;

use outline::{build_outline, Module};

#[derive(Clone)]
struct Config {
    api_key: String,
    api_base: String,
    model: String,
    mode: Mode,
    concurrency: usize,
    limit: Option<usize>,
    target_words: u32,
    out_dir: PathBuf,
    max_retries: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Cluster,
    Single,
    DryRun,
}

fn parse_args() -> Result<Config> {
    let mut mode = Mode::Cluster;
    let mut concurrency = 64usize;
    let mut limit: Option<usize> = None;
    let mut target_words = 1500u32;
    let mut out_dir = PathBuf::from("doc_out");
    let mut max_retries = 3usize;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                i += 1;
                mode = match args.get(i).map(|s| s.as_str()) {
                    Some("cluster") => Mode::Cluster,
                    Some("single") => Mode::Single,
                    Some("dry-run") => Mode::DryRun,
                    other => return Err(anyhow!("unknown --mode: {:?}", other)),
                };
            }
            "--concurrency" => {
                i += 1;
                concurrency = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(64);
            }
            "--limit" => {
                i += 1;
                limit = args.get(i).and_then(|s| s.parse().ok());
            }
            "--target-words" => {
                i += 1;
                target_words = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(780);
            }
            "--out" => {
                i += 1;
                out_dir = args.get(i).map(PathBuf::from).unwrap_or(out_dir);
            }
            "--max-retries" => {
                i += 1;
                max_retries = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(3);
            }
            other => return Err(anyhow!("unknown arg: {}", other)),
        }
        i += 1;
    }

    let api_key = std::env::var("ANTLET_API_KEY")
        .map_err(|_| anyhow!("ANTLET_API_KEY not set"))?;
    let api_base = std::env::var("ANTLET_API_BASE")
        .unwrap_or_else(|_| "https://api.minimaxi.com/v1".to_string())
        .trim_end_matches('/')
        .to_string();
    let model = std::env::var("ANTLET_MODEL").unwrap_or_else(|_| "MiniMax-M2.5".to_string());

    Ok(Config {
        api_key,
        api_base,
        model,
        mode,
        concurrency,
        limit,
        target_words,
        out_dir,
        max_retries,
    })
}

// ---- minimal OpenAI-compatible chat client (no tools needed) ----

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    message: ApiMessage,
}
#[derive(Deserialize)]
struct ApiMessage {
    #[serde(default)]
    content: String,
}

async fn write_module(
    http: &reqwest::Client,
    cfg: &Config,
    m: &Module,
) -> Result<String> {
    let system = format!(
        "你是《从经典强化学习到 Agentic RL》技术专著的特约撰稿专家，负责撰写其中一个知识模块。\
         你所在的位置：{} / {} / {}。\
         要求：用准确、严谨、有深度的中文撰写，面向有一定数学基础的读者。\
         必须包含必要的数学公式（使用 LaTeX，行内用 $...$，块级用 $$...$$）。\
         只输出该模块的 markdown 正文，不要寒暄、不要重复标题层级以外的内容。",
        m.part, m.chapter, m.section
    );

    let points = m
        .points
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{}. {}", i + 1, p))
        .collect::<Vec<_>>()
        .join("\n");

    let user = format!(
        "请为以下模块撰写约 {} 字的高质量技术内容：\n\n\
         模块标题：{}\n\n\
         撰写要点：\n{}\n\n\
         输出格式：以 `#### {}` 作为四级标题开头，随后是正文。\
         内容要充实、专业，覆盖上述要点，并包含至少一个关键公式。",
        m.target_words, m.title, points, m.title
    );

    let url = format!("{}/chat/completions", cfg.api_base);
    let body = json!({
        "model": cfg.model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ]
    });

    let resp = http
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("HTTP {} {}", status, truncate(&text, 200)));
    }

    let parsed: ChatResponse = resp.json().await?;
    let content = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| anyhow!("empty choices"))?;

    if content.trim().is_empty() {
        return Err(anyhow!("empty content"));
    }
    Ok(content)
}

fn module_path(out_dir: &PathBuf, m: &Module) -> PathBuf {
    out_dir.join("modules").join(format!("{}.md", m.id))
}

fn char_count(s: &str) -> usize {
    s.chars().filter(|c| !c.is_whitespace()).count()
}

async fn process_module(
    http: reqwest::Client,
    cfg: Config,
    m: Module,
) -> (String, Result<usize>) {
    let path = module_path(&cfg.out_dir, &m);

    // Resume support: skip already-written non-empty files.
    if let Ok(existing) = std::fs::read_to_string(&path) {
        if char_count(&existing) > 50 {
            return (m.id.clone(), Ok(char_count(&existing)));
        }
    }

    let mut last_err = None;
    for attempt in 0..=cfg.max_retries {
        match write_module(&http, &cfg, &m).await {
            Ok(content) => {
                if let Some(parent) = path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                if let Err(e) = tokio::fs::write(&path, &content).await {
                    return (m.id.clone(), Err(anyhow!("write failed: {}", e)));
                }
                return (m.id.clone(), Ok(char_count(&content)));
            }
            Err(e) => {
                last_err = Some(e);
                if attempt < cfg.max_retries {
                    let backoff = 2u64.pow(attempt as u32);
                    tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                }
            }
        }
    }
    (m.id.clone(), Err(last_err.unwrap_or_else(|| anyhow!("unknown error"))))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = parse_args()?;
    let mut modules = build_outline(cfg.target_words);
    if let Some(limit) = cfg.limit {
        modules.truncate(limit);
    }

    let total = modules.len();
    let est_words = total as u32 * cfg.target_words;

    println!("=== Antlet doc-cluster demo ===");
    println!("mode        : {}", match cfg.mode {
        Mode::Cluster => format!("cluster (concurrency={})", cfg.concurrency),
        Mode::Single => "single (sequential)".to_string(),
        Mode::DryRun => "dry-run".to_string(),
    });
    println!("modules     : {}", total);
    println!("target/mod  : {} 字", cfg.target_words);
    println!("est. total  : {} 字 (~{:.2} 万字)", est_words, est_words as f64 / 10000.0);
    println!("model       : {}", cfg.model);
    println!("out_dir     : {}", cfg.out_dir.display());
    println!("================================\n");

    if cfg.mode == Mode::DryRun {
        // Print the outline tree summary.
        let mut last_part = String::new();
        let mut last_chapter = String::new();
        let mut last_section = String::new();
        for m in &modules {
            if m.part != last_part {
                println!("\n{}", m.part);
                last_part = m.part.clone();
                last_chapter.clear();
                last_section.clear();
            }
            if m.chapter != last_chapter {
                println!("  {}", m.chapter);
                last_chapter = m.chapter.clone();
                last_section.clear();
            }
            if m.section != last_section {
                println!("    {}", m.section);
                last_section = m.section.clone();
            }
            println!("      [{}] {}", m.id, m.title);
        }
        println!("\nTOTAL MODULES: {}  EST WORDS: {} (~{:.2} 万字)",
            total, est_words, est_words as f64 / 10000.0);
        return Ok(());
    }

    tokio::fs::create_dir_all(cfg.out_dir.join("modules")).await?;

    let http = reqwest::Client::builder()
        .pool_max_idle_per_host(cfg.concurrency.max(16))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_default();

    let done = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let total_chars = Arc::new(AtomicU64::new(0));
    let start = Instant::now();

    match cfg.mode {
        Mode::Cluster => {
            let sem = Arc::new(Semaphore::new(cfg.concurrency));
            let mut handles = Vec::with_capacity(total);
            for m in modules.clone() {
                let permit_sem = sem.clone();
                let http = http.clone();
                let cfg = cfg.clone();
                let done = done.clone();
                let failed = failed.clone();
                let total_chars = total_chars.clone();
                handles.push(tokio::spawn(async move {
                    let _permit = permit_sem.acquire_owned().await.unwrap();
                    let (id, res) = process_module(http, cfg, m).await;
                    report(&id, res, &done, &failed, &total_chars, total, &start);
                }));
            }
            for h in handles {
                let _ = h.await;
            }
        }
        Mode::Single => {
            for m in modules.clone() {
                let (id, res) = process_module(http.clone(), cfg.clone(), m).await;
                report(&id, res, &done, &failed, &total_chars, total, &start);
            }
        }
        Mode::DryRun => unreachable!(),
    }

    let elapsed = start.elapsed();
    let chars = total_chars.load(Ordering::SeqCst);
    let ok = done.load(Ordering::SeqCst);
    let fail = failed.load(Ordering::SeqCst);

    // Assemble final document.
    let final_path = cfg.out_dir.join("FINAL_DOCUMENT.md");
    let assembled_chars = assemble(&cfg, &modules, &final_path).await?;

    let secs = elapsed.as_secs_f64();
    println!("\n=== RUN COMPLETE ===");
    println!("mode            : {}", match cfg.mode {
        Mode::Cluster => format!("cluster (concurrency={})", cfg.concurrency),
        Mode::Single => "single".to_string(),
        Mode::DryRun => "".to_string(),
    });
    println!("modules ok      : {}/{}", ok, total);
    println!("modules failed  : {}", fail);
    println!("wall-clock      : {:.1} s ({:.2} min)", secs, secs / 60.0);
    println!("chars written   : {} (~{:.2} 万字)", chars, chars as f64 / 10000.0);
    println!("assembled doc   : {} 字 -> {}", assembled_chars, final_path.display());
    if secs > 0.0 {
        println!("throughput      : {:.2} modules/s, {:.0} 字/s", ok as f64 / secs, chars as f64 / secs);
    }
    println!("====================");

    Ok(())
}

fn report(
    id: &str,
    res: Result<usize>,
    done: &Arc<AtomicUsize>,
    failed: &Arc<AtomicUsize>,
    total_chars: &Arc<AtomicU64>,
    total: usize,
    start: &Instant,
) {
    match res {
        Ok(chars) => {
            let n = done.fetch_add(1, Ordering::SeqCst) + 1;
            total_chars.fetch_add(chars as u64, Ordering::SeqCst);
            if n % 10 == 0 || n == total {
                let secs = start.elapsed().as_secs_f64();
                let cumulative = total_chars.load(Ordering::SeqCst);
                println!(
                    "[{:>4}/{}] {} ok ({} 字) | elapsed {:.0}s | total {:.2} 万字 | {:.1} mod/s",
                    n, total, id, chars, secs, cumulative as f64 / 10000.0, n as f64 / secs.max(0.001)
                );
            }
        }
        Err(e) => {
            failed.fetch_add(1, Ordering::SeqCst);
            eprintln!("[FAIL] {} : {}", id, truncate(&e.to_string(), 160));
        }
    }
}

async fn assemble(cfg: &Config, modules: &[Module], final_path: &PathBuf) -> Result<usize> {
    let mut doc = String::new();
    doc.push_str("# 从经典强化学习到 Agentic RL：技术专著\n\n");
    doc.push_str(&format!(
        "> 本文档由 Antlet 智能体集群生成，共 {} 个知识模块，每个模块由一个独立 agent 实例撰写。\n\n",
        modules.len()
    ));

    let mut last_part = String::new();
    let mut last_chapter = String::new();
    let mut last_section = String::new();
    let mut total = 0usize;

    for m in modules {
        if m.part != last_part {
            doc.push_str(&format!("\n# {}\n\n", m.part));
            last_part = m.part.clone();
            last_chapter.clear();
            last_section.clear();
        }
        if m.chapter != last_chapter {
            doc.push_str(&format!("\n## {}\n\n", m.chapter));
            last_chapter = m.chapter.clone();
            last_section.clear();
        }
        if m.section != last_section {
            doc.push_str(&format!("\n### {}\n\n", m.section));
            last_section = m.section.clone();
        }
        let path = module_path(&cfg.out_dir, m);
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => {
                total += char_count(&content);
                doc.push_str(content.trim());
                doc.push_str("\n\n");
            }
            Err(_) => {
                doc.push_str(&format!("> _[模块 {} 缺失]_\n\n", m.id));
            }
        }
    }

    tokio::fs::write(final_path, &doc).await?;
    Ok(total)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}
