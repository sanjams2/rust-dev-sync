use std::path::Path;

use async_trait::async_trait;
use notify::EventKind;

pub mod rsyncer;

pub type Result = std::result::Result<(), String>;

// Send/Sync is required to be able to move a syncer to a tokio thread context. I need to figure out why
#[async_trait]
pub trait Syncer: std::marker::Sync + std::marker::Send {
    async fn sync(&self, workspace_path: &Path, file_path: &Path, kind: EventKind) -> Result;
}
