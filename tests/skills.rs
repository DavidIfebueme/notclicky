use std::path::PathBuf;

#[test]
fn loader_loads_skills_from_resources() {
    let resources_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let skills_dir = resources_dir.join("skills");
    if !skills_dir.exists() {
        return;
    }

    let mut loader = notclicky::skills::loader::SkillLoader::new(skills_dir);
    let skills = loader.load_all().unwrap();
    assert!(!skills.is_empty(), "should load at least one skill");
}

#[test]
fn loader_excludes_mac_skills() {
    let resources_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let skills_dir = resources_dir.join("skills");
    if !skills_dir.exists() {
        return;
    }

    let mut loader = notclicky::skills::loader::SkillLoader::new(skills_dir);
    let skills = loader.load_all().unwrap();

    let mac_skills = [
        "apple-notes",
        "apple-reminders",
        "imessage",
        "findmy",
        "maps",
    ];
    for skill in &skills {
        assert!(
            !mac_skills.contains(&skill.name.as_str()),
            "Mac skill {} should be excluded",
            skill.name
        );
    }
}

#[test]
fn loader_gets_skill_by_name() {
    let resources_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let skills_dir = resources_dir.join("skills");
    if !skills_dir.exists() {
        return;
    }

    let mut loader = notclicky::skills::loader::SkillLoader::new(skills_dir);
    loader.load_all().unwrap();

    let skill = loader.get("x11-linux");
    assert!(skill.is_some(), "x11-linux skill should exist");
    assert!(skill.unwrap().content.contains("xdotool"));
}

#[test]
fn loader_finds_by_tag() {
    let resources_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let skills_dir = resources_dir.join("skills");
    if !skills_dir.exists() {
        return;
    }

    let mut loader = notclicky::skills::loader::SkillLoader::new(skills_dir);
    loader.load_all().unwrap();

    let results = loader.find_by_tag("API");
    assert!(!results.is_empty(), "should find skills with API tag");
}

#[test]
fn suggestion_engine_loads_rules() {
    let resources_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let rules_path = resources_dir.join("skill-suggestion-rules.json");
    if !rules_path.exists() {
        return;
    }

    let engine = notclicky::skills::suggestion::SuggestionEngine::from_file(&rules_path).unwrap();
    let defaults = engine.get_default_suggestions();
    assert!(!defaults.is_empty(), "should have default suggestions");
}

#[test]
fn suggestion_engine_matches_lm_studio() {
    let resources_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let rules_path = resources_dir.join("skill-suggestion-rules.json");
    if !rules_path.exists() {
        return;
    }

    let engine = notclicky::skills::suggestion::SuggestionEngine::from_file(&rules_path).unwrap();
    let suggestions = engine.suggest_for_window("LM Studio - Local Models");
    assert!(
        !suggestions.is_empty(),
        "should suggest skills for LM Studio"
    );
}

#[test]
fn skill_context_builds_prompt() {
    let resources_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    if !resources_dir.join("skills").exists() {
        return;
    }

    let ctx = notclicky::skills::context::SkillContext::new(resources_dir);
    let prompt = ctx.build_system_prompt(Some("LM Studio - Local Models"));
    assert!(
        !prompt.is_empty(),
        "prompt should not be empty for LM Studio"
    );
}

#[test]
fn linux_skill_count() {
    let resources_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let skills_dir = resources_dir.join("skills");
    if !skills_dir.exists() {
        return;
    }

    let mut loader = notclicky::skills::loader::SkillLoader::new(skills_dir);
    let skills = loader.load_all().unwrap();
    assert!(
        skills.len() >= 37,
        "should load at least 37 Linux-applicable skills, got {}",
        skills.len()
    );
}
