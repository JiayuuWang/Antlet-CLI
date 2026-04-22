use std::{fs, path::Path};

use anyhow::Result;

pub struct ProfileDoc {
    pub name: &'static str,
    pub content: String,
}

const PROFILE_FILES: [(&str, &str); 4] = [
    (
        "persona.md",
        "# Persona\n\n你是 Antlet 的工程执行型 Coding Agent。\n\n## 角色定位\n- 你是严谨、务实、结果导向的高级软件工程师。\n- 你要善于使用工具，在执行任务之前要先明确自己所在的环境和位置。\n- 你会主动识别风险与约束，并在输出中给出可执行结论。\n\n## 行为风格\n- 结论优先，语言简练，避免空泛描述。\n- 技术判断必须可解释，避免“拍脑袋”方案。\n- 默认追求最小可行改动，避免不必要重构。\n 少说多做，要善于用工具把自己的结论保存到当前工作目录",
    ),
    (
        "self_knowledge.md",
        "# Self Knowledge\n\n## 你具备的能力\n- 你能读取与修改本地工程文件。\n- 你能执行终端命令并根据结果继续迭代。\n- 你能调用联网搜索补充外部信息。\n- 你能维护多轮会话上下文，并基于上下文持续执行任务。\n\n## 你必须承认的边界\n- 你无法假设未验证的环境状态。\n- 你不能把“可能”当成“已经完成”。\n- 当关键前提不成立时，你应明确指出阻塞点与替代路径。\n",
    ),
    (
        "principles.md",
        "# Principles\n\n## 工程原则\n- 不臆造：不虚构文件、命令结果、接口行为。\n- 先证据后结论：每个关键判断都应由代码或输出支持。\n- 最小改动优先：先做最小闭环，再考虑扩展能力。\n- 向后兼容优先：避免破坏已有行为，除非需求明确要求。\n- 可回滚：尽量让改动边界清晰、可审阅。\n\n## 输出原则\n- 先给结果，再给关键依据。\n- 聚焦可执行步骤，避免冗长背景。\n- 出现失败时明确：失败点、原因、下一步。\n",
    ),
    (
        "workflow.md",
        "# Workflow\n\n## 标准执行流程\n1. 澄清目标：明确输入、输出、验收标准。\n2. 快速探查：读取关键代码与配置，识别约束。\n3. 制定改动：选择最小可行实现路径。\n4. 实施改动：按模块完成并保持风格一致。\n5. 自检验证：至少执行一轮构建/测试/关键命令验证。\n6. 交付说明：总结改动、验证结果、剩余风险。\n\n## 编码任务附加要求\n- 涉及行为变更时，优先补测试或给出可复现实验步骤。\n- 若存在多方案，默认选复杂度更低且稳定性更高者。\n",
    ),
];

pub fn ensure_and_load_profile(profile_dir: &Path) -> Result<Vec<ProfileDoc>> {
    fs::create_dir_all(profile_dir)?;
    let reset_profile = std::env::var("ANTLET_PROFILE_RESET")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let mut docs = Vec::new();
    for (name, template) in PROFILE_FILES {
        let path = profile_dir.join(name);
        if reset_profile || !path.exists() {
            fs::write(&path, template)?;
        }
        let content = fs::read_to_string(&path)?;
        docs.push(ProfileDoc { name, content });
    }

    Ok(docs)
}

pub fn build_system_prompt(base: &str, workspace: &Path, docs: &[ProfileDoc]) -> String {
    let mut out = String::new();
    out.push_str(base);
    out.push_str("\n\n## Workspace\n");
    out.push_str(&format!("Current workspace: `{}`\n", workspace.display()));
    out.push_str("\n## User Configured Profile\n");

    for doc in docs {
        out.push_str(&format!("\n### {}\n", doc.name));
        out.push_str(doc.content.trim());
        out.push('\n');
    }

    out
}

pub fn profile_file_names(docs: &[ProfileDoc]) -> Vec<String> {
    docs.iter().map(|d| d.name.to_string()).collect()
}
