use super::RepoPath;
use crate::{article::WikiArticle, user_storage::UserAccount};
use git2::Repository;

pub struct ReadOnly<'a> {
    pub(super) repo_path: &'a RepoPath,
    pub(super) repo: Repository,
}

impl ReadOnly<'_> {
    pub fn get_current_oid_for_article(
        &self,
        article: &WikiArticle,
    ) -> Result<Option<git2::Oid>, super::Error> {
        match super::repo_head(&self.repo)? {
            None => Ok(None),
            Some(head) => {
                let head_commit = head.peel_to_commit().unwrap();
                let tree = head_commit.tree()?;
                let tree_path = self.repo_path.tree_path(&article.path);

                super::get_blob_oid(&tree, tree_path)
            }
        }
    }

    pub fn history(&self, article: &WikiArticle) -> Result<Vec<HistoryEntry>, super::Error> {
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
                if super::get_blob_oid(&tree, tree_path)?.is_some() {
                    let signature = commit.author();
                    ret.push(HistoryEntry {
                        user: UserAccount {
                            name: try_to_string(signature.name()),
                            email: try_to_string(signature.email()),
                        },
                        date: time::OffsetDateTime::from_unix_timestamp(commit.time().seconds()),
                        summary: try_to_string(commit.summary()),
                    });
                }
            }
        }

        Ok(ret)
    }
}

// FIXME: javascript can't parse ISO dates. I have no words.
//pub struct ISOUtcDate(time::OffsetDateTime);
//
//impl ISOUtcDate {
//    pub fn from_unix(time: i64) -> Self {
//        Self(time::OffsetDateTime::from_unix_timestamp(time))
//    }
//}
//
//impl std::fmt::Display for ISOUtcDate {
//    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//        self.0.lazy_format("%FT%TZ").fmt(formatter)
//    }
//}

fn try_to_string(opt: Option<&str>) -> String {
    opt.map(ToOwned::to_owned).unwrap_or_else(String::new)
}

pub struct HistoryEntry {
    pub user: crate::user_storage::UserAccount,
    pub date: time::OffsetDateTime,
    pub summary: String,
}

