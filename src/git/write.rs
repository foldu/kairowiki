use super::{RepoPath, TreePath};
use crate::{api, api::EditSubmit, article::WikiArticle, serde::Oid, user_storage::UserAccount};
use git2::{Repository, ResetType, Signature};
use tokio::sync::MutexGuard;

pub(super) fn write_and_commit_file(
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

pub(super) struct CommitInfo<'a> {
    pub path: TreePath<'a>,
    pub signature: git2::Signature<'a>,
    pub msg: &'a str,
}

pub struct RepoLock<'a> {
    pub(super) path: &'a RepoPath,
    pub(super) repo: MutexGuard<'a, Repository>,
}

fn read_entry(repo: &git2::Repository, ent: git2::IndexEntry) -> Result<String, git2::Error> {
    let ent = repo.find_blob(ent.id)?;
    // FIXME:
    Ok(String::from_utf8(ent.content().to_vec()).unwrap())
}

impl<'a> RepoLock<'a> {
    pub fn commit_article(
        &self,
        article: &WikiArticle,
        account: &UserAccount,
        edit: &EditSubmit,
    ) -> Result<api::Commit, super::Error> {
        let tree_path = self.path.tree_path(&article.path);

        let signature = Signature::now(&account.name, &account.email).unwrap();

        let commit_info = CommitInfo {
            path: tree_path,
            signature,
            msg: &edit.commit_msg,
        };

        let head = super::repo_head(&self.repo)?.expect("Empty repo");
        let head_commit = head.peel_to_commit().unwrap();
        let head_tree = head_commit.tree()?;

        let head_article_oid = super::get_tree_path(&head_tree, &tree_path)?.map(|art| art.id());
        let current_oid = match (head_article_oid, edit.oid) {
            // somebody changed the article while editing
            (Some(current), Some(ancestor)) if current != ancestor.0 => Some(current),
            // somebody created article with the same name while editing
            (Some(current), None) => Some(current),
            // no diff, just overwrite/add the file to the tree
            _ => None,
        };

        match current_oid {
            Some(current_oid) => {
                let ancestor_commit = self.repo.find_commit(edit.rev.0)?;
                let ancestor_tree = ancestor_commit.tree()?;
                let new_blob = self.repo.blob(edit.markdown.as_bytes())?;
                let mut new_tree = self.repo.treebuilder(Some(&head_tree))?;
                new_tree.insert(tree_path.as_ref(), new_blob, 0o100644)?;
                let new_tree = self.repo.find_tree(new_tree.write()?)?;

                let index = self
                    .repo
                    .merge_trees(&ancestor_tree, &head_tree, &new_tree, None)?;

                if index.has_conflicts() {
                    let conflict = index.conflicts()?.next().unwrap()?;
                    let ancestor = conflict
                        .ancestor
                        .map(|ent| read_entry(&self.repo, ent))
                        .transpose()?;
                    let ours = read_entry(&self.repo, conflict.our.unwrap())?;
                    let theirs = read_entry(&self.repo, conflict.their.unwrap())?;
                    Ok(api::Commit::Conflict {
                        ancestor,
                        ours,
                        theirs,
                        oid: Oid(current_oid),
                        rev: Oid(head_commit.id()),
                    })
                } else {
                    let entry = index.get_path(tree_path.as_ref(), 0).unwrap();

                    let merged = self.repo.find_blob(entry.id)?;
                    // TODO: handle if the TreeEntry is a subtree(directory)

                    Ok(api::Commit::Merged {
                        // TODO: handle binary files
                        merged: String::from_utf8(merged.content().to_vec()).unwrap(),
                        oid: Oid(current_oid),
                        rev: Oid(head_commit.id()),
                    })
                }
            }
            None => {
                write_and_commit_file(
                    &self.repo,
                    Some((head_commit, &head_tree)),
                    &commit_info,
                    &edit.markdown,
                )?;
                Ok(api::Commit::NoConflict)
            }
        }
    }
}
