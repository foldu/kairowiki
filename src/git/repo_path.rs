use std::path::{Path, PathBuf};

pub struct RepoPath(PathBuf);

impl RepoPath {
    pub fn new(path: PathBuf) -> Self {
        RepoPath(path)
    }

    pub fn open(&self) -> Result<git2::Repository, git2::Error> {
        git2::Repository::open(&self.0)
    }
}

impl RepoPath {
    pub fn tree_path<'a>(&self, path: &'a Path) -> TreePath<'a> {
        TreePath(path.strip_prefix(&self.0).expect("Invalid git repo path"))
    }
}

#[derive(derive_more::AsRef, Copy, Clone)]
pub struct TreePath<'a>(&'a Path);
