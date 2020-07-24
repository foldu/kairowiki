use crate::{
    article::{ArticlePath, ArticleTitle},
    serde::Oid,
};
use git2::{Repository, TreeWalkMode, TreeWalkResult};

pub struct ReadOnly {
    pub(super) repo: Repository,
}

impl ReadOnly {
    pub fn article_at_rev(
        &self,
        rev: git2::Oid,
        path: &ArticlePath,
    ) -> Result<Option<(Oid, String)>, super::Error> {
        let commit = match self.repo.find_commit(rev) {
            // FIXME: does it return NotFound on not found commit?
            Err(e) if e.code() == git2::ErrorCode::NotFound => {
                return Ok(None);
            }
            other => other,
        };

        let commit = commit?;

        let tree = commit.tree()?;
        match super::get_as_blob(&self.repo, &tree, &path)? {
            // FIXME: return error when blob not utf-8
            Some(blob) => {
                let content = String::from_utf8(blob.content().to_vec()).ok();
                Ok(content.map(|content| (Oid(blob.id()), content)))
            }
            None => Ok(None),
        }
    }

    pub fn head(&self) -> Result<git2::Reference<'_>, super::Error> {
        Ok(super::repo_head(&self.repo)?.expect("Uninitialized repo"))
    }

    pub fn history(&self, article_path: &ArticlePath) -> Result<Vec<HistoryEntry>, super::Error> {
        let mut rev_walk = self.repo.revwalk()?;
        rev_walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)?;
        rev_walk.push_head()?;

        let mut ret = Vec::new();
        let mut last_oid = None;
        for commit_oid in rev_walk {
            let commit_oid = commit_oid?;
            if let Ok(commit) = self.repo.find_commit(commit_oid) {
                let tree = commit.tree()?;
                if let Ok(Some(blob_oid)) = super::get_blob_oid(&tree, &article_path) {
                    if Some(blob_oid) != last_oid {
                        let signature = commit.author();
                        ret.push(HistoryEntry {
                            user: Signature {
                                name: try_to_string(signature.name()),
                                email: try_to_string(signature.email()),
                            },
                            date: ISOUtcDate::from_unix(commit.time().seconds()),
                            summary: try_to_string(commit.summary()),
                            rev: commit_oid,
                        });
                        last_oid = Some(blob_oid);
                    }
                }
            }
        }

        ret.reverse();

        Ok(ret)
    }

    fn entry_to_article_info(&self, entry: &git2::TreeEntry) -> Option<(ArticleTitle, String)> {
        let title = entry
            .name()
            .and_then(|path| ArticleTitle::from_path(path).ok())?;

        let obj = entry.to_object(&self.repo).ok()?;
        let content = obj
            .as_blob()
            .and_then(|blob| std::str::from_utf8(blob.content()).ok())?;

        Some((title, content.to_owned()))
    }

    pub fn traverse_head_tree(
        &self,
        mut f: impl FnMut(ArticleTitle, String),
    ) -> Result<(), super::Error> {
        let head = self.head()?;
        let tree = head.peel_to_commit()?.tree()?;

        tree.walk(TreeWalkMode::PreOrder, |_some_str, entry| {
            if let Some((title, content)) = self.entry_to_article_info(entry) {
                f(title, content)
            }

            TreeWalkResult::Ok
        })?;

        Ok(())
    }
}

pub struct ISOUtcDate(time::OffsetDateTime);

impl ISOUtcDate {
    pub fn from_unix(time: i64) -> Self {
        Self(time::OffsetDateTime::from_unix_timestamp(time))
    }
}

impl std::fmt::Display for ISOUtcDate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.lazy_format("%FT%H:%M:%SZ").fmt(formatter)
    }
}

fn try_to_string(opt: Option<&str>) -> String {
    opt.map(ToOwned::to_owned).unwrap_or_else(String::new)
}

pub struct Signature {
    pub name: String,
    pub email: String,
}

pub struct HistoryEntry {
    pub user: Signature,
    pub date: ISOUtcDate,
    pub summary: String,
    pub rev: git2::Oid,
}
