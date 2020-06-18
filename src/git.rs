mod repo_path;

use crate::{api, article::WikiArticle, handlers::api::EditSubmit, user_storage::UserAccount};
use git2::{Repository, ResetType, Signature};
use repo_path::{RepoPath, TreePath};
use std::path::PathBuf;
use tokio::sync::{Mutex, MutexGuard};

// FIXME: better error messages
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("Could not open or init repository in path {}: {}", path.display(), err)]
    RepoOpen { path: PathBuf, err: git2::Error },
}

pub struct Repo {
    path: RepoPath,
    repo: Mutex<Repository>,
}

impl Repo {
    pub fn open_or_init(path: PathBuf) -> Result<Self, Error> {
        let repo = match Repository::open(&path) {
            Err(e) if e.code() == git2::ErrorCode::NotFound => Repository::init(&path),
            other => other,
        };

        match repo {
            Ok(repo) => {
                // when current worktree clean update it on push
                // TODO: check if this actually works the way I think it does
                repo.config()
                    // FIXME: handle error
                    .and_then(|mut cfg| {
                        cfg.set_str("receive.denyCurrentBranch", "updateInstead")
                    })?;

                Ok(Self {
                    path: RepoPath::new(path),
                    repo: Mutex::new(repo),
                })
            }
            Err(e) => Err(Error::RepoOpen { path, err: e }),
        }
    }

    pub fn read(&self) -> Result<ReadOnly, Error> {
        let repo = self.path.open()?;
        Ok(ReadOnly {
            repo,
            repo_path: &self.path,
        })
    }

    pub async fn write<'a>(&'a self) -> RepoLock<'a> {
        RepoLock {
            path: &self.path,
            repo: self.repo.lock().await,
        }
    }
}

fn repo_head<'a>(repo: &'a Repository) -> Result<Option<git2::Reference<'a>>, git2::Error> {
    match repo.head() {
        Ok(head) => Ok(Some(head)),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => Ok(None),
        Err(e) => Err(e),
    }
}

fn get_tree_path<'a>(
    tree: &'a git2::Tree,
    path: TreePath,
) -> Result<Option<git2::TreeEntry<'a>>, git2::Error> {
    match tree.get_path(path.as_ref()) {
        Ok(ent) => Ok(Some(ent)),
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn get_blob_oid<'a>(tree: &'a git2::Tree, tree_path: TreePath) -> Result<Option<git2::Oid>, Error> {
    match get_tree_path(&tree, tree_path)? {
        Some(ent) if ent.kind() == Some(git2::ObjectType::Blob) => Ok(Some(ent.id())),
        _ => Ok(None),
    }
}

fn get_as_blob<'a>(
    repo: &'a Repository,
    tree: &git2::Tree,
    path: TreePath,
) -> Result<Option<git2::Blob<'a>>, git2::Error> {
    let ent = match get_tree_path(tree, path)? {
        Some(ent) => ent,
        None => return Ok(None),
    };

    let obj = ent.to_object(repo)?;
    match obj.into_blob() {
        Ok(blob) => Ok(Some(blob)),
        Err(_) => Ok(None),
    }
}

pub struct ReadOnly<'a> {
    repo_path: &'a RepoPath,
    repo: Repository,
}

impl ReadOnly<'_> {
    pub fn get_current_oid_for_article(
        &self,
        article: &WikiArticle,
    ) -> Result<Option<git2::Oid>, Error> {
        match repo_head(&self.repo)? {
            None => Ok(None),
            Some(head) => {
                let head_commit = head.peel_to_commit().unwrap();
                let tree = head_commit.tree()?;
                let tree_path = self.repo_path.tree_path(&article.path);

                get_blob_oid(&tree, tree_path)
            }
        }
    }

    pub fn history(&self, article: &WikiArticle) -> Result<Vec<HistoryEntry>, Error> {
        let tree_path = self.repo_path.tree_path(&article.path);

        let mut rev_walk = self.repo.revwalk()?;
        rev_walk.set_sorting(git2::Sort::TIME)?;
        match repo_head(&self.repo)? {
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
                if let Some(_) = get_blob_oid(&tree, tree_path)? {
                    let signature = commit.author();
                    ret.push(HistoryEntry {
                        user: UserAccount {
                            name: try_to_string(signature.name()),
                            email: try_to_string(signature.email()),
                        },
                        date: time::OffsetDateTime::from_unix_timestamp(commit.time().seconds()),
                        summary: commit
                            .summary()
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| String::new()),
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
    opt.map(ToOwned::to_owned).unwrap_or_else(|| String::new())
}

pub struct HistoryEntry {
    pub user: crate::user_storage::UserAccount,
    pub date: time::OffsetDateTime,
    pub summary: String,
}

fn write_and_commit_file(
    repo: &Repository,
    previous: Option<(git2::Commit, &git2::Tree)>,
    commit_info: &CommitInfo,
    new: &str,
) -> Result<(), git2::Error> {
    let article_oid = repo.blob(new.as_bytes())?;
    //let previous_tree = previous.map(|(_, ref tree)| tree);
    let mut tree_builder = repo.treebuilder(match previous {
        Some((_, ref tree)) => Some(tree),
        None => None,
    })?;
    tree_builder.insert(commit_info.path.as_ref(), article_oid, 0o100644)?;
    let tree_oid = tree_builder.write()?;
    let tree = repo.find_tree(tree_oid).unwrap();

    let mut parent_commits = vec![];
    if let Some((ref commit, _)) = previous {
        parent_commits.push(commit);
    }

    let commit_oid = repo.commit(
        Some("HEAD"),
        &commit_info.signature,
        &commit_info.signature,
        &commit_info.msg,
        &tree,
        &parent_commits,
    )?;
    let commit = repo.find_commit(commit_oid)?;
    repo.reset(commit.as_object(), ResetType::Hard, None)
}

struct CommitInfo<'a> {
    path: TreePath<'a>,
    signature: git2::Signature<'a>,
    msg: &'a str,
}

pub struct RepoLock<'a> {
    path: &'a RepoPath,
    repo: MutexGuard<'a, Repository>,
}

impl<'a> RepoLock<'a> {
    pub fn commit_article(
        &self,
        article: &WikiArticle,
        account: &UserAccount,
        edit: EditSubmit,
    ) -> Result<api::Commit, Error> {
        let tree_path = self.path.tree_path(&article.path);

        let signature = Signature::now(&account.name, &account.email).unwrap();
        let commit_msg = format!("Update {}", article.title);

        let commit_info = CommitInfo {
            path: tree_path,
            signature,
            msg: &commit_msg,
        };

        match repo_head(&self.repo)? {
            Some(head) => {
                let head_commit = head.peel_to_commit().unwrap();
                let head_tree = head_commit.tree()?;

                let head_article = get_tree_path(&head_tree, tree_path)?;
                let has_diff = match (head_article, edit.oid) {
                    // somebody changed the article while editing
                    (Some(current), Some(start)) if current.id() != start => Some(current),
                    // somebody created article with the same name while editing
                    (Some(current), None) => Some(current),
                    // no diff, just overwrite/add the file to the tree
                    _ => None,
                };

                match has_diff {
                    Some(ent) => {
                        // FIXME: use get_as_blob here
                        // TODO: handle if the TreeEntry is a subtree(directory)
                        let obj = ent.to_object(&self.repo)?;
                        let blob = obj.as_blob().unwrap();
                        Ok(api::Commit::Diff {
                            // TODO: handle binary files
                            a: String::from_utf8(blob.content().to_vec()).unwrap(),
                            b: edit.markdown,
                        })
                    }
                    None => {
                        write_and_commit_file(
                            &self.repo,
                            Some((head_commit, &head_tree)),
                            &commit_info,
                            &edit.markdown,
                        )?;
                        Ok(api::Commit::Ok)
                    }
                }
            }
            // repo has no commits
            None => {
                write_and_commit_file(&self.repo, None, &commit_info, &edit.markdown)?;

                Ok(api::Commit::Ok)
            }
        }
    }
}

impl warp::reject::Reject for Error {}
