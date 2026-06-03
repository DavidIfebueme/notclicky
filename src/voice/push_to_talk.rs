use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait GlobalHotkey: Send + Sync {
    fn register(&self, modifiers: Vec<&str>, key: Option<&str>) -> Result<()>;
    #[allow(dead_code)]
    fn unregister(&self) -> Result<()>;
    fn is_pressed(&self) -> bool;
}
