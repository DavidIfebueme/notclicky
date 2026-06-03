use crate::skills::loader::{Skill, SkillLoader};
use crate::skills::suggestion::SuggestionEngine;
use std::path::PathBuf;

pub struct SkillContext {
    loader: SkillLoader,
    suggestion_engine: Option<SuggestionEngine>,
}

impl SkillContext {
    pub fn new(resources_dir: PathBuf) -> Self {
        let skills_dir = resources_dir.join("skills");
        let rules_path = resources_dir.join("skill-suggestion-rules.json");

        let mut loader = SkillLoader::new(skills_dir);
        let _ = loader.load_all();

        let suggestion_engine = SuggestionEngine::from_file(&rules_path).ok();

        Self {
            loader,
            suggestion_engine,
        }
    }

    pub fn build_system_prompt(&self, window_title: Option<&str>) -> String {
        let mut parts = Vec::new();

        if let Some(engine) = &self.suggestion_engine {
            if let Some(title) = window_title {
                let suggestions = engine.suggest_for_window(title);
                if !suggestions.is_empty() {
                    let suggestion_text: Vec<String> = suggestions
                        .iter()
                        .map(|s| format!("- {}: {}", s.chip_title, s.detail))
                        .collect();
                    parts.push(format!("Suggested skills for active window:\n{}", suggestion_text.join("\n")));
                }
            }
        }

        if let Some(title) = window_title {
            let relevant_skills = self.find_relevant_skills(title);
            if !relevant_skills.is_empty() {
                let skill_sections: Vec<String> = relevant_skills
                    .iter()
                    .map(|s| format!("## {}\n{}", s.name, s.content))
                    .collect();
                parts.push(format!("Active skills:\n\n{}", skill_sections.join("\n\n")));
            }
        }

        parts.join("\n\n")
    }

    #[allow(dead_code)]
    pub fn get_skill(&self, name: &str) -> Option<&Skill> {
        self.loader.get(name)
    }

    #[allow(dead_code)]
    pub fn list_skills(&self) -> Vec<&Skill> {
        self.loader.list()
    }

    #[allow(dead_code)]
    pub fn get_suggestions(&self, window_title: &str) -> Vec<crate::skills::suggestion::Suggestion> {
        if let Some(engine) = &self.suggestion_engine {
            engine.suggest_for_window(window_title)
        } else {
            vec![]
        }
    }

    fn find_relevant_skills(&self, window_title: &str) -> Vec<&Skill> {
        let lower = window_title.to_lowercase();
        let mut relevant = Vec::new();

        for skill in self.loader.list() {
            for tag in &skill.tags {
                if lower.contains(&tag.to_lowercase()) {
                    relevant.push(skill);
                    break;
                }
            }
        }

        if let Some(engine) = &self.suggestion_engine {
            let suggestions = engine.suggest_for_window(window_title);
            for suggestion in &suggestions {
                if let Some(skill) = self.loader.get(&suggestion.id) {
                    if !relevant.iter().any(|s| s.name == skill.name) {
                        relevant.push(skill);
                    }
                }
            }
        }

        relevant
    }
}
