use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

use notclicky::app::Secrets;

#[test]
fn parse_secrets_env_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("secrets.env");
    fs::write(
        &path,
        "ZAI_API_KEY=sk-test-123\nDEEPGRAM_API_KEY=dg-abc\n# comment\n\nANTHROPIC_API_KEY=ant-key\n",
    ).unwrap();

    let values = notclicky::app::parse_env_file(&path).unwrap();
    assert_eq!(values.get("ZAI_API_KEY").unwrap(), "sk-test-123");
    assert_eq!(values.get("DEEPGRAM_API_KEY").unwrap(), "dg-abc");
    assert_eq!(values.get("ANTHROPIC_API_KEY").unwrap(), "ant-key");
    assert_eq!(values.len(), 3);
}

#[test]
fn env_var_fills_missing_secrets() {
    let key = "ASSEMBLYAI_API_KEY";
    let existing = std::env::var(key).ok();
    unsafe { std::env::set_var(key, "test-assembly-key"); }
    let secrets = Secrets::load().unwrap();
    assert_eq!(secrets.get(key).unwrap(), "test-assembly-key");
    match existing {
        Some(v) => unsafe { std::env::set_var(key, v); },
        None => unsafe { std::env::remove_var(key); },
    }
}

#[test]
fn require_missing_key_returns_error() {
    let key = "NOTCLICKY_TEST_MISSING_KEY";
    unsafe { std::env::remove_var(key); }
    let secrets = Secrets::load().unwrap();
    let err = secrets.require(key).unwrap_err();
    assert!(err.to_string().contains(key));
    assert!(err.to_string().contains("secrets.env"));
}

#[test]
fn save_and_reread_secrets() {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join("notclicky");
    fs::create_dir_all(&config_dir).unwrap();

    let mut secrets = Secrets::load().unwrap();
    secrets.set("NOTCLICKY_TEST_SAVE", "test-value-42").ok();

    let path = config_dir.join("secrets.env");
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("NOTCLICKY_TEST_SAVE=test-value-42"));
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
