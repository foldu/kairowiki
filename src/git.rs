// FIXME: better error messages
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Can't write to git repo: {0}")]
    Write(std::io::Error),

    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),
}

impl warp::reject::Reject for Error {}

pub fn commit_article(
    git_repo: &std::path::Path,
    article: &crate::article::WikiArticle,
    account: &crate::user_storage::UserAccount,
    new_article: &crate::forms::NewArticle,
) -> Result<(), Error> {
    let repo = git2::Repository::open(git_repo)?;
    let mut sleep_time = std::time::Duration::from_millis(10);
    // I just hope acquiring the index locks it
    // TODO: lookup if this actually locks it
    let index = loop {
        match repo.index() {
            Err(e) if e.code() == git2::ErrorCode::Locked => {
                std::thread::sleep(sleep_time);
                sleep_time = std::cmp::min(sleep_time * 3, std::time::Duration::from_millis(300));
            }
            other => break other,
        }
    };
    let mut index = index?;

    // TODO: hard reset repository if this fails
    std::fs::write(&*article.path, &new_article.markdown).map_err(Error::Write)?;

    let relative_to_repo = article
        .path
        .strip_prefix(git_repo)
        .expect("Invalid git repo path");
    index.add_path(&relative_to_repo)?;
    let oid = index.write_tree()?;

    let tree = repo.find_tree(oid)?;

    // TODO: add newtype around account that guarantees no null bytes
    let signature = git2::Signature::now(&account.name, &account.email).unwrap();
    let commit_msg = format!("Update {}", article.title);
    let head = repo.head();

    match head {
        // FIXME: damn you borrowck
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &commit_msg,
                &tree,
                &[],
            )?;
        }
        Ok(head) => {
            let head = head.peel_to_commit().unwrap();
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &commit_msg,
                &tree,
                &[&head],
            )?;
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}
