use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionRules {
    #[serde(default)]
    pub default_suggestions: Vec<Suggestion>,
    #[serde(default)]
    pub app_rules: Vec<AppRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub id: String,
    pub title: String,
    pub detail: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub install_prompt: String,
    #[serde(default)]
    pub chip_title: String,
    #[serde(default)]
    pub system_image: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRule {
    pub id: String,
    #[serde(default)]
    pub app_matches: Vec<String>,
    #[serde(default)]
    pub suggestions: Vec<Suggestion>,
}

pub struct SuggestionEngine {
    rules: SuggestionRules,
    app_rules_index: HashMap<String, Vec<String>>,
}

impl SuggestionEngine {
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let rules: SuggestionRules = serde_json::from_str(&content)?;
        let mut index = HashMap::new();

        for rule in &rules.app_rules {
            for app_match in &rule.app_matches {
                let entry = index.entry(app_match.to_lowercase()).or_insert_with(Vec::new);
                entry.push(rule.id.clone());
            }
        }

        Ok(Self {
            rules,
            app_rules_index: index,
        })
    }

    pub fn get_default_suggestions(&self) -> &[Suggestion] {
        &self.rules.default_suggestions
    }

    pub fn suggest_for_window(&self, window_title: &str) -> Vec<Suggestion> {
        let lower = window_title.to_lowercase();
        let mut suggestions = Vec::new();

        for (app_match, rule_ids) in &self.app_rules_index {
            if lower.contains(app_match) {
                for rule in &self.rules.app_rules {
                    if rule_ids.contains(&rule.id) {
                        suggestions.extend(rule.suggestions.clone());
                    }
                }
            }
        }

        suggestions
    }

    pub fn suggest_for_app(&self, app_name: &str) -> Vec<Suggestion> {
        self.suggest_for_window(app_name)
    }
}
