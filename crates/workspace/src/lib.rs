mod file;
mod workspace;

use dashmap::DashMap;
use std::{path::PathBuf, sync::Arc};
use anyhow::Result;

use crate::workspace::{Workspace, WorkspaceId};

pub trait WorkspaceManager: Send + Sync + 'static {
    fn lookup(&self, id: &WorkspaceId) -> Option<Workspace>;
    fn add_root(&self, root_path: PathBuf) -> Result<WorkspaceId> {
        Ok()
    }
}

#[derive(Debug, Default)]
pub struct LspWorkspaceManager {
    workspaces: Arc<DashMap<WorkspaceId, Workspace>>,
}

impl LspWorkspaceManager {
    pub fn new() -> Self {
        Self {
            workspaces: Arc::new(DashMap::new()),
        }
    }
}

impl WorkspaceManager for LspWorkspaceManager {
    fn lookup(&self, id: &WorkspaceId) -> Option<Workspace> {
        self.workspaces.get(id).map(|ws| ws.clone())
    }
}
