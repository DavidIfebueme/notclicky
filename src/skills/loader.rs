use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MAC_SKILLS: &[&str] = &["apple-notes", "apple-reminders", "imessage", "findmy", "maps"];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub prerequisites: SkillPrerequisites,
    #[serde(default)]
    pub metadata: SkillMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillPrerequisites {
    #[serde(default)]
    pub env_vars: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillMetadata {
    #[serde(default, alias = "openclicky")]
    pub notclicky: SkillNotclickyMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillNotclickyMeta {
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub prerequisites: SkillPrerequisites,
    pub tags: Vec<String>,
    pub content: String,
    pub source_path: PathBuf,
}

pub struct SkillLoader {
    skills_dir: PathBuf,
    skills: HashMap<String, Skill>,
}

impl SkillLoader {
    pub fn new(skills_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            skills: HashMap::new(),
        }
    }

    pub fn load_all(&mut self) -> Result<Vec<Skill>> {
        if !self.skills_dir.exists() {
            return Ok(vec![]);
        }

        let mut loaded = Vec::new();

        for entry in std::fs::read_dir(&self.skills_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    if let Some(skill) = self.load_skill_file(&skill_md)? {
                        loaded.push(skill);
                    }
                }
            } else if path.extension().map_or(false, |e| e == "md") {
                if let Some(skill) = self.load_skill_file(&path)? {
                    loaded.push(skill);
                }
            }
        }

        for skill in &loaded {
            self.skills.insert(skill.name.clone(), skill.clone());
        }

        Ok(loaded)
    }

    fn load_skill_file(&self, path: &Path) -> Result<Option<Skill>> {
        let content = std::fs::read_to_string(path)?;
        let (frontmatter, body) = parse_frontmatter(&content)?;

        let name = frontmatter.name.clone();

        if MAC_SKILLS.contains(&name.as_str()) {
            return Ok(None);
        }

        Ok(Some(Skill {
            name: frontmatter.name,
            description: frontmatter.description,
            version: frontmatter.version,
            author: frontmatter.author,
            license: frontmatter.license,
            prerequisites: frontmatter.prerequisites,
            tags: frontmatter.metadata.notclicky.tags,
            content: body,
            source_path: path.to_path_buf(),
        }))
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
            .collect()
    }

    pub fn check_prerequisites(&self, skill: &Skill) -> Vec<String> {
        let mut missing = Vec::new();

        for cmd in &skill.prerequisites.commands {
            if which_command(cmd).is_none() {
                missing.push(format!("command: {}", cmd));
            }
        }

        for var in &skill.prerequisites.env_vars {
            if std::env::var(var).is_err() {
                missing.push(format!("env_var: {}", var));
            }
        }

        missing
    }
}

fn parse_frontmatter(content: &str) -> Result<(SkillFrontmatter, String)> {
    if !content.starts_with("---") {
        return Ok((SkillFrontmatter::default(), content.to_string()));
    }

    let end = content[3..].find("---").map(|i| i + 3).unwrap_or(0);
    if end == 0 {
        return Ok((SkillFrontmatter::default(), content.to_string()));
    }

    let yaml_str = &content[3..end];
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(yaml_str)?;
    let body = content[end + 3..].trim().to_string();

    Ok((frontmatter, body))
}

fn which_command(cmd: &str) -> Option<PathBuf> {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .map(PathBuf::from)
}
