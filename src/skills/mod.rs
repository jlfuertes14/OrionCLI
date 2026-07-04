use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPrompt {
    pub inject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub skill: SkillInfo,
    pub prompt: SkillPrompt,
    pub aliases: Option<HashMap<String, String>>,
}

pub struct SkillRegistry {
    pub skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn load_defaults() -> Self {
        let mut registry = SkillRegistry {
            skills: HashMap::new(),
        };
        let _ = registry.scan_skills_dir();
        registry
    }

    pub fn scan_skills_dir(&mut self) -> Result<()> {
        let skills_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not find home directory"))?
            .join(".orion")
            .join("skills");

        if !skills_dir.exists() {
            fs::create_dir_all(&skills_dir)?;
            // Create a default example skill
            let example_toml = r#"[skill]
name = "web-researcher"
description = "Expert web research with structured output"
version = "1.0"

[prompt]
inject = """
You are an expert web researcher. Always cite your sources.
When asked to research a topic, use browser_read and web_search tools
to gather information from multiple sources before answering.
"""
"#;
            let _ = fs::write(skills_dir.join("web-researcher.toml"), example_toml);
        }

        if let Ok(entries) = fs::read_dir(skills_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(skill) = toml::from_str::<Skill>(&content) {
                                self.skills.insert(skill.skill.name.clone(), skill);
                            }
                        }
                    }
                }
            }
        }

        // Scan .agents/skills for npx skills ecosystem compatibility
        if let Some(home) = dirs::home_dir() {
            let agents_skills_dir = home.join(".agents").join("skills");
            if let Ok(entries) = fs::read_dir(agents_skills_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let skill_md_path = path.join("SKILL.md");
                        if skill_md_path.exists() {
                            if let Ok(content) = fs::read_to_string(&skill_md_path) {
                                if let Some(skill) = parse_skill_md(&content) {
                                    self.skills.insert(skill.skill.name.clone(), skill);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }
}

pub async fn download_skill(github_repo: &str) -> Result<Vec<String>> {
    let parts: Vec<&str> = github_repo.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid GitHub repository format. Use: owner/repo"));
    }
    let owner = parts[0];
    let repo = parts[1];

    let client = reqwest::Client::builder()
        .user_agent("OrionBot-CLI")
        .build()?;

    let mut downloaded_names = Vec::new();

    // 1. Try fetching the /skills directory contents via GitHub API
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/contents/skills",
        owner, repo
    );
    let resp = client.get(&api_url).send().await?;

    if resp.status().is_success() {
        if let Ok(items) = resp.json::<Vec<serde_json::Value>>().await {
            for item in items {
                if item.get("type").and_then(|t| t.as_str()) == Some("dir") {
                    if let Some(dir_name) = item.get("name").and_then(|n| n.as_str()) {
                        // Attempt to fetch SKILL.md from this directory
                        match fetch_and_save_skill(
                            &client,
                            owner,
                            repo,
                            &format!("skills/{}", dir_name),
                            dir_name,
                        )
                        .await
                        {
                            Ok(name) => downloaded_names.push(name),
                            Err(_) => {} // skip if no SKILL.md in this folder
                        }
                    }
                }
            }
        }
    }

    // 2. If no skills were found in /skills directory, try root SKILL.md
    if downloaded_names.is_empty() {
        match fetch_and_save_skill(&client, owner, repo, "", repo).await {
            Ok(name) => downloaded_names.push(name),
            Err(e) => {
                return Err(anyhow!(
                    "Could not find SKILL.md in root or /skills/ subdirectories: {}",
                    e
                ));
            }
        }
    }

    Ok(downloaded_names)
}

async fn fetch_and_save_skill(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    subpath: &str,
    save_filename: &str,
) -> Result<String> {
    let path_part = if subpath.is_empty() {
        "SKILL.md".to_string()
    } else {
        format!("{}/SKILL.md", subpath)
    };

    // Try main branch first
    let mut url = format!(
        "https://raw.githubusercontent.com/{}/{}/main/{}",
        owner, repo, path_part
    );
    let mut resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        // Try master branch fallback
        url = format!(
            "https://raw.githubusercontent.com/{}/{}/master/{}",
            owner, repo, path_part
        );
        resp = client.get(&url).send().await?;
    }

    if !resp.status().is_success() {
        return Err(anyhow!(
            "SKILL.md not found in branch (HTTP {})",
            resp.status()
        ));
    }

    let content = resp.text().await?;
    let (name, description, body) = parse_skill_markdown(&content)?;

    // Format as TOML compatible with our Skill registry
    let skill_toml = format!(
        r#"[skill]
name = "{}"
description = "{}"
version = "1.0"

[prompt]
inject = """
{}
"""
"#,
        name,
        description.replace('"', "\\\""),
        body.replace('\\', "\\\\").replace('"', "\\\"")
    );

    let skills_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not resolve home directory"))?
        .join(".orion")
        .join("skills");

    std::fs::create_dir_all(&skills_dir)?;
    let dest_path = skills_dir.join(format!("{}.toml", save_filename));
    std::fs::write(&dest_path, skill_toml)?;

    Ok(name)
}

fn parse_skill_markdown(content: &str) -> Result<(String, String, String)> {
    let parts: Vec<&str> = content.split("---").collect();
    if parts.len() < 3 {
        return Err(anyhow!("Missing YAML frontmatter in SKILL.md"));
    }

    let frontmatter = parts[1];
    let body = parts[2..].join("---");

    let mut name = String::new();
    let mut description = String::new();

    for line in frontmatter.lines() {
        let line = line.trim();
        if line.starts_with("name:") {
            name = line["name:".len()..].trim().trim_matches('"').to_string();
        } else if line.starts_with("description:") {
            description = line["description:".len()..]
                .trim()
                .trim_matches('"')
                .to_string();
        }
    }

    if name.is_empty() {
        return Err(anyhow!("Missing 'name' field in frontmatter"));
    }

    Ok((name, description, body.trim().to_string()))
}

fn parse_skill_md(content: &str) -> Option<Skill> {
    if let Ok((name, description, inject)) = parse_skill_markdown(content) {
        Some(Skill {
            skill: SkillInfo {
                name,
                description,
                version: "1.0".to_string(),
            },
            prompt: SkillPrompt { inject },
            aliases: None,
        })
    } else {
        None
    }
}
