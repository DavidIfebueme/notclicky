use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub title: String,
    pub path: String,
    pub content: String,
    pub aliases: Vec<String>,
}

pub struct WikiManager {
    wiki_dir: PathBuf,
    pages: HashMap<String, WikiPage>,
}

impl WikiManager {
    pub fn new(wiki_dir: PathBuf) -> Self {
        Self {
            wiki_dir,
            pages: HashMap::new(),
        }
    }

    pub fn load(&mut self) -> Result<()> {
        if !self.wiki_dir.exists() {
            return Ok(());
        }

        let wiki_dir = self.wiki_dir.clone();
        self.load_dir(&wiki_dir, "")?;
        Ok(())
    }

    fn load_dir(&mut self, dir: &Path, prefix: &str) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name().unwrap().to_string_lossy();
                let sub_prefix = if prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{}/{}", prefix, name)
                };
                self.load_dir(&path, &sub_prefix)?;
            } else if path.extension().map_or(false, |e| e == "md") {
                let content = std::fs::read_to_string(&path)?;
                let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
                let page_path = if prefix.is_empty() {
                    format!("{}.md", file_name)
                } else {
                    format!("{}/{}.md", prefix, file_name)
                };

                let (title, aliases) = parse_page_metadata(&content, &file_name);

                let page = WikiPage {
                    title,
                    path: page_path,
                    content,
                    aliases,
                };

                self.pages.insert(page.title.to_lowercase(), page);
            }
        }
        Ok(())
    }

    pub fn import_seed(&mut self, seed_dir: &Path) -> Result<()> {
        if !seed_dir.exists() {
            return Ok(());
        }

        copy_dir_recursive(seed_dir, &self.wiki_dir)?;
        self.load()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get(&self, title: &str) -> Option<&WikiPage> {
        self.pages.get(&title.to_lowercase())
    }

    #[allow(dead_code)]
    pub fn search(&self, query: &str) -> Vec<&WikiPage> {
        let lower = query.to_lowercase();
        self.pages
            .values()
            .filter(|page| {
                page.title.to_lowercase().contains(&lower)
                    || page.aliases.iter().any(|a| a.to_lowercase().contains(&lower))
                    || page.content.to_lowercase().contains(&lower)
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn create(&mut self, title: &str, content: &str, category: Option<&str>) -> Result<()> {
        let file_name = title.to_lowercase().replace(' ', "-");
        let page_path = match category {
            Some(cat) => {
                let dir = self.wiki_dir.join(cat);
                std::fs::create_dir_all(&dir)?;
                format!("{}/{}.md", cat, file_name)
            }
            None => format!("{}.md", file_name),
        };

        let full_path = self.wiki_dir.join(&page_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full_path, content)?;

        let page = WikiPage {
            title: title.to_string(),
            path: page_path,
            content: content.to_string(),
            aliases: vec![],
        };

        self.pages.insert(title.to_lowercase(), page);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update(&mut self, title: &str, content: &str) -> Result<()> {
        let page = match self.pages.get_mut(&title.to_lowercase()) {
            Some(p) => p,
            None => anyhow::bail!("Page not found: {}", title),
        };

        let full_path = self.wiki_dir.join(&page.path);
        std::fs::write(&full_path, content)?;
        page.content = content.to_string();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete(&mut self, title: &str) -> Result<()> {
        let page = match self.pages.remove(&title.to_lowercase()) {
            Some(p) => p,
            None => anyhow::bail!("Page not found: {}", title),
        };

        let full_path = self.wiki_dir.join(&page.path);
        if full_path.exists() {
            std::fs::remove_file(&full_path)?;
        }
        Ok(())
    }

    pub fn list(&self) -> Vec<&WikiPage> {
        self.pages.values().collect()
    }
}

fn parse_page_metadata(content: &str, file_name: &str) -> (String, Vec<String>) {
    let mut title = file_name.replace('-', " ");
    let mut aliases = Vec::new();

    for line in content.lines() {
        if line.starts_with("# ") {
            title = line[2..].to_string();
            break;
        }
    }

    if let Some(start) = content.find("also: ") {
        let rest = &content[start + 6..];
        if let Some(end) = rest.find('\n') {
            aliases = rest[..end]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    (title, aliases)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            if !dst_path.exists() {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
    }

    Ok(())
}
