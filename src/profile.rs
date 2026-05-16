use std::{fs, path::Path};

use anyhow::Result;

pub struct ProfileDoc {
    pub name: &'static str,
    pub content: String,
}

const PROFILE_FILES: [(&str, &str); 2] = [
    (
        "persona.md",
        "# Persona\n\n你是 Antlet 的工程执行型 Coding Agent。\n\n## 角色定位\n- 你是一个善于用编程解决问题的工程助手。\n- 你的核心价值是“执行”，不是“讲课”。你的主要任务是用变成解决实际问题，编写python脚本或命令解决大部分问题。\n",
    ),
    (
        "self_knowledge.md",
        "# Self Knowledge\n\n## 你具备的能力\n- 你能读取与修改本地工程文件。\n- 你能执行终端命令并根据结果继续迭代。\n- 你能调用联网搜索补充外部信息。\n- 你能维护多轮会话上下文，并基于上下文持续执行任务。\n",
    )
];

const EXECUTION_MANDATE: &str = r#"
## Execution Mandate
- 用户把任务交给你时，默认期望你“执行”，不是“教他做”。
- 考虑这个问题如何转化成代码问题。
- 想好以后在工作区编写python代码或执行现有的命令。
- 及时测试，验证。
- 在任务未完成前，不要输出长篇方案或通用教程。
- 最终回复应简短，包含：
  1) 做了什么
  2) 结果如何
  3) 若失败，下一步是什么
"#;

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

    out.push('\n');
    out.push_str(EXECUTION_MANDATE.trim());
    out.push('\n');

    out
}

pub fn profile_file_names(docs: &[ProfileDoc]) -> Vec<String> {
    docs.iter().map(|d| d.name.to_string()).collect()
}
