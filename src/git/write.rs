use super::{RepoPath, TreePath};
use crate::{api, article::WikiArticle, api::EditSubmit, user_storage::UserAccount};
use git2::{Repository, ResetType, Signature};
use tokio::sync::MutexGuard;

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
    pub(super) path: &'a RepoPath,
    pub(super) repo: MutexGuard<'a, Repository>,
}

impl<'a> RepoLock<'a> {
    pub fn commit_article(
        &self,
        article: &WikiArticle,
        account: &UserAccount,
        edit: EditSubmit,
    ) -> Result<api::Commit, super::Error> {
        let tree_path = self.path.tree_path(&article.path);

        let signature = Signature::now(&account.name, &account.email).unwrap();
        let commit_msg = format!("Update {}", article.title);

        let commit_info = CommitInfo {
            path: tree_path,
            signature,
            msg: &commit_msg,
        };

        match super::repo_head(&self.repo)? {
            Some(head) => {
                let head_commit = head.peel_to_commit().unwrap();
                let head_tree = head_commit.tree()?;

                let head_article = super::get_tree_path(&head_tree, tree_path)?;
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
                        let obj = ent.to_object(&self.repo)?;
                        // TODO: handle if the TreeEntry is a subtree(directory)
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
