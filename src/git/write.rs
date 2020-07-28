use crate::{api, api::EditSubmit, article::ArticlePath, serde::Oid, user_storage::UserAccount};
use git2::{IndexEntry, Repository, Signature};
use smallvec::SmallVec;
use std::{convert::TryFrom, os::unix::prelude::*, time::SystemTime};
use tokio::sync::MutexGuard;

trait IndexExt {
    fn new_for_path<P>(path: P, file_size: u32) -> Self
    where
        P: AsRef<std::path::Path>;
}

impl IndexExt for git2::IndexEntry {
    fn new_for_path<P>(path: P, file_size: u32) -> Self
    where
        P: AsRef<std::path::Path>,
    {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            // cannot fail
            .unwrap();

        let timestamp = git2::IndexTime::new(
            i32::try_from(now.as_secs())
                .expect("It is the year 2038 and libgit2 still uses signed 32 bit timestamps"),
            0,
        );

        git2::IndexEntry {
            ctime: timestamp,
            mtime: timestamp,
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: nix::unistd::getuid().as_raw(),
            gid: nix::unistd::getgid().as_raw(),
            file_size,
            id: git2::Oid::zero(),
            flags: 0,
            flags_extended: 0,
            path: path.as_ref().as_os_str().as_bytes().to_vec(),
        }
    }
}

pub(super) fn write_and_commit_file(
    repo: &Repository,
    previous_commit: Option<&git2::Commit>,
    commit_info: &CommitInfo,
    new: &str,
) -> Result<(), git2::Error> {
    let path = commit_info.path.as_ref();
    let mut index = repo.index()?;
    if let Some(commit) = previous_commit {
        let tree = commit.tree()?;
        index.read_tree(&tree)?;
    }

    index.add_frombuffer(&IndexEntry::new_for_path(path, 0), new.as_bytes())?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let mut parent_commits = SmallVec::<[_; 1]>::new();
    if let Some(commit) = previous_commit {
        parent_commits.push(commit);
    }

    repo.commit(
        Some("HEAD"),
        &commit_info.signature,
        &commit_info.signature,
        &commit_info.msg,
        &tree,
        &parent_commits,
    )?;

    Ok(())
}

pub(super) struct CommitInfo<'a> {
    pub path: &'a ArticlePath,
    pub signature: git2::Signature<'a>,
    pub msg: &'a str,
}

pub struct RepoLock<'a> {
    //pub(super) path: &'a RepoPath,
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
        article_path: &ArticlePath,
        account: &UserAccount,
        edit: &EditSubmit,
    ) -> Result<api::Commit, super::Error> {
        let signature = Signature::now(&account.name, &account.email).unwrap();

        let commit_info = CommitInfo {
            path: &article_path,
            signature,
            msg: &edit.commit_msg,
        };

        let head = super::repo_head(&self.repo)?.expect("Empty repo");
        let head_commit = head.peel_to_commit().unwrap();
        let head_tree = head_commit.tree()?;

        let head_article_oid = super::get_tree_path(&head_tree, &article_path)?.map(|art| art.id());
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
                new_tree.insert(article_path.as_ref(), new_blob, 0o100644)?;
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
                    let entry = index.get_path(article_path.as_ref(), 0).unwrap();

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
                    Some(&head_commit),
                    &commit_info,
                    &edit.markdown,
                )?;
                Ok(api::Commit::NoConflict)
            }
        }
    }
}
