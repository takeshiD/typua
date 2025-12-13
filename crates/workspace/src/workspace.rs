use std::path::PathBuf;

use crate::file::{FileId, FileText};
use dashmap::DashMap;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct WorkspaceId {
    id: String,
}

#[derive(Debug, Clone)]
pub enum WorkspaceKind {
    Project,
    Library,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    root: FileId,
    files: DashMap<FileId, FileText>,
    kind: WorkspaceKind,
}

#[derive(Debug, Clone)]
pub struct WorkspaceIdInterner {
    path_to_id: DashMap<PathBuf, WorkspaceId>,
    id_to_path: DashMap<WorkspaceId, PathBuf>,
}
