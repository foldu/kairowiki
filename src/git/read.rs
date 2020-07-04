use super::RepoPath;
use crate::{
    article::{ArticlePath, WikiArticle},
    user_storage::UserAccount,
};
use git2::Repository;

pub struct ReadOnly<'a> {
    pub(super) repo_path: &'a RepoPath,
    pub(super) repo: Repository,
}

impl<'a> ReadOnly<'a> {
    pub fn article_at_rev(
        &'a self,
        rev: git2::Oid,
        path: &ArticlePath,
    ) -> Result<Option<String>, super::Error> {
        let tree_path = self.repo_path.tree_path(path);
        let commit = match self.repo.find_commit(rev) {
            // FIXME: does it return NotFound on not found commit?
            Err(e) if e.code() == git2::ErrorCode::NotFound => {
                return Ok(None);
            }
            other => other,
        };

        let commit = commit?;

        let tree = commit.tree()?;
        let blob = super::get_as_blob(&self.repo, &tree, &tree_path)?;
        Ok(blob.and_then(|blob| String::from_utf8(blob.content().to_vec()).ok()))
    }

    pub fn head(&'a self) -> Result<git2::Reference<'a>, super::Error> {
        Ok(super::repo_head(&self.repo)?.expect("Uninitialized repo"))
    }

    pub fn oid_for_article(
        &'a self,
        rev: &'a git2::Reference,
        article: &WikiArticle,
    ) -> Result<Option<git2::Oid>, super::Error> {
        let commit = rev.peel_to_commit().unwrap();
        let tree = commit.tree()?;
        let tree_path = self.repo_path.tree_path(&article.path);

        super::get_blob_oid(&tree, &tree_path)
    }

    pub fn history(&'a self, article: &WikiArticle) -> Result<Vec<HistoryEntry>, super::Error> {
        let tree_path = self.repo_path.tree_path(&article.path);

        let mut rev_walk = self.repo.revwalk()?;
        rev_walk.set_sorting(git2::Sort::TIME)?;
        match super::repo_head(&self.repo)? {
            Some(_) => {
                rev_walk.push_head()?;
            }
            _ => return Ok(Vec::new()),
        };

        let mut ret = Vec::new();
        for oid in rev_walk {
            let oid = oid?;
            if let Ok(commit) = self.repo.find_commit(oid) {
                let tree = commit.tree()?;
                if super::get_blob_oid(&tree, &tree_path)?.is_some() {
                    let signature = commit.author();
                    ret.push(HistoryEntry {
                        user: UserAccount {
                            name: try_to_string(signature.name()),
                            email: try_to_string(signature.email()),
                        },
                        date: ISOUtcDate::from_unix(commit.time().seconds()),
                        summary: try_to_string(commit.summary()),
                        rev: oid,
                    });
                }
            }
        }

        Ok(ret)
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
        self.0.lazy_format("%FT%TZ").fmt(formatter)
    }
}

fn try_to_string(opt: Option<&str>) -> String {
    opt.map(ToOwned::to_owned).unwrap_or_else(String::new)
}

pub struct HistoryEntry {
    pub user: crate::user_storage::UserAccount,
    pub date: ISOUtcDate,
    pub summary: String,
    pub rev: git2::Oid,
}
