use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use anyhow::{Result, anyhow};

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
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }
}

pub async fn download_skill(github_repo: &str) -> Result<String> {
    let parts: Vec<&str> = github_repo.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid GitHub repository format. Use: owner/repo"));
    }
    let owner = parts[0];
    let repo = parts[1];

    let client = reqwest::Client::builder()
        .user_agent("OrionBot-CLI")
        .build()?;

    // Try main branch first
    let mut url = format!("https://raw.githubusercontent.com/{}/{}/main/skill.toml", owner, repo);
    let mut resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        // Try master branch fallback
        url = format!("https://raw.githubusercontent.com/{}/{}/master/skill.toml", owner, repo);
        resp = client.get(&url).send().await?;
    }

    if !resp.status().is_success() {
        return Err(anyhow!(
            "Failed to download skill.toml from main or master branch (HTTP {})",
            resp.status()
        ));
    }

    let content = resp.text().await?;
    
    // Validate it is valid TOML for our Skill struct
    let _val: Skill = toml::from_str(&content)
        .map_err(|e| anyhow!("Downloaded file is not a valid skill TOML: {}", e))?;

    let skills_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not resolve home directory"))?
        .join(".orion")
        .join("skills");

    std::fs::create_dir_all(&skills_dir)?;
    let dest_path = skills_dir.join(format!("{}.toml", repo));
    std::fs::write(&dest_path, content)?;

    Ok(repo.to_string())
}

