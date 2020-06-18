use crate::{api, article::WikiArticle, handlers::api::EditSubmit, user_storage::UserAccount};
use git2::{Repository, ResetType, Signature};
use std::path::{Path, PathBuf};
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
    path: PathBuf,
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
                    path,
                    repo: Mutex::new(repo),
                })
            }
            Err(e) => Err(Error::RepoOpen { path, err: e }),
        }
    }

    pub fn read(&self) -> Result<ReadOnly, Error> {
        let repo = Repository::open(&self.path)?;
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
    path: &Path,
) -> Result<Option<git2::TreeEntry<'a>>, git2::Error> {
    match tree.get_path(path) {
        Ok(ent) => Ok(Some(ent)),
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct ReadOnly<'a> {
    repo_path: &'a Path,
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
                let tree_path = article
                    .path
                    .strip_prefix(&self.repo_path)
                    .expect("Invalid git repo path");

                // jesus christ FIXME
                match tree.get_path(tree_path) {
                    Ok(entry) => match entry.to_object(&self.repo)?.as_blob() {
                        Some(blob) => Ok(Some(blob.id())),
                        None => panic!(),
                    },
                    Err(_) => Ok(None),
                }
            }
        }
    }

    pub fn history_of_file(&self) -> () {
        todo!()
    }
}

pub struct RepoLock<'a> {
    path: &'a Path,
    repo: MutexGuard<'a, Repository>,
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
    tree_builder.insert(commit_info.path, article_oid, 0o100644)?;
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
    path: &'a Path,
    signature: git2::Signature<'a>,
    msg: &'a str,
}

impl<'a> RepoLock<'a> {
    pub fn commit_article(
        &self,
        article: &WikiArticle,
        account: &UserAccount,
        edit: EditSubmit,
    ) -> Result<api::Commit, Error> {
        let tree_path = article
            .path
            .strip_prefix(&self.path)
            .expect("Invalid git repo path");

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
