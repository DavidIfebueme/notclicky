use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn wiki_create_and_read() {
    let dir = TempDir::new().unwrap();
    let mut wiki = notclicky::memory::wiki::WikiManager::new(PathBuf::from(dir.path()));

    wiki.create("Test Page", "Hello world", None).unwrap();
    let page = wiki.get("Test Page").unwrap();
    assert_eq!(page.content, "Hello world");
    assert_eq!(page.title, "Test Page");
}

#[test]
fn wiki_update() {
    let dir = TempDir::new().unwrap();
    let mut wiki = notclicky::memory::wiki::WikiManager::new(PathBuf::from(dir.path()));

    wiki.create("My Page", "v1", None).unwrap();
    wiki.update("My Page", "v2").unwrap();
    assert_eq!(wiki.get("My Page").unwrap().content, "v2");
}

#[test]
fn wiki_delete() {
    let dir = TempDir::new().unwrap();
    let mut wiki = notclicky::memory::wiki::WikiManager::new(PathBuf::from(dir.path()));

    wiki.create("To Delete", "bye", None).unwrap();
    wiki.delete("To Delete").unwrap();
    assert!(wiki.get("To Delete").is_none());
}

#[test]
fn wiki_search() {
    let dir = TempDir::new().unwrap();
    let mut wiki = notclicky::memory::wiki::WikiManager::new(PathBuf::from(dir.path()));

    wiki.create("Rust Programming", "Rust is a systems language", None).unwrap();
    wiki.create("Python Guide", "Python is interpreted", None).unwrap();

    let results = wiki.search("rust");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Rust Programming");
}

#[test]
fn wiki_case_insensitive_get() {
    let dir = TempDir::new().unwrap();
    let mut wiki = notclicky::memory::wiki::WikiManager::new(PathBuf::from(dir.path()));

    wiki.create("Hello World", "content", None).unwrap();
    assert!(wiki.get("hello world").is_some());
    assert!(wiki.get("HELLO WORLD").is_some());
}

#[test]
fn wiki_seed_import() {
    let resources_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let seed_dir = resources_dir.join("wiki");
    if !seed_dir.exists() {
        return;
    }

    let dir = TempDir::new().unwrap();
    let mut wiki = notclicky::memory::wiki::WikiManager::new(PathBuf::from(dir.path()));
    wiki.import_seed(&seed_dir).unwrap();

    let pages = wiki.list();
    assert!(!pages.is_empty(), "seed should import at least one page");
}

#[test]
fn conversation_history_adds_exchanges() {
    let mut history = notclicky::memory::conversation::ConversationHistory::new();
    history.add("hello".to_string(), "hi there".to_string());
    history.add("how are you".to_string(), "fine".to_string());
    assert_eq!(history.exchanges().len(), 2);
}

#[test]
fn conversation_history_compacts() {
    let mut history = notclicky::memory::conversation::ConversationHistory::new();

    for i in 0..20 {
        let long_msg = "x".repeat(300);
        history.add(format!("user {}", i), long_msg.clone());
    }

    assert!(history.exchanges().len() <= 8, "should compact to at most 8 exchanges, got {}", history.exchanges().len());
    assert!(!history.archive().is_empty(), "archive should contain compacted exchanges");
}

#[test]
fn conversation_history_prompt_context() {
    let mut history = notclicky::memory::conversation::ConversationHistory::new();
    history.add("hello".to_string(), "hi".to_string());

    let ctx = history.to_prompt_context();
    assert!(ctx.contains("hello"));
    assert!(ctx.contains("hi"));
}

#[test]
fn persistent_memory_read_write() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("memory.md");

    let mut mem = notclicky::memory::conversation::PersistentMemory::new(path.clone());
    mem.set("learned something".to_string()).unwrap();

    let mem2 = notclicky::memory::conversation::PersistentMemory::new(path);
    assert_eq!(mem2.get(), "learned something");
}

#[test]
fn persistent_memory_append() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("memory.md");

    let mut mem = notclicky::memory::conversation::PersistentMemory::new(path);
    mem.append("line 1").unwrap();
    mem.append("line 2").unwrap();
    assert!(mem.get().contains("line 1"));
    assert!(mem.get().contains("line 2"));
}
