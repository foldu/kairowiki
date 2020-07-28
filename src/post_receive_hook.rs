use crate::{
    ipc::{send_update, Update, SOCK_PATH},
    serde::Oid,
};
use std::io::{stdin, BufRead};

pub async fn run() -> Result<(), anyhow::Error> {
    let mut ln = String::new();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    while stdin.read_line(&mut ln)? != 0 {
        let split = ln.split_whitespace().collect::<Vec<_>>();
        if let &[parent_commit_id, new_commit_id, "refs/heads/master"] = &split[..] {
            let parent_commit_id = Oid::parse(parent_commit_id)?;
            let new_commit_id = Oid::parse(new_commit_id)?;
            send_update(
                SOCK_PATH,
                &Update {
                    parent_commit_id,
                    new_commit_id,
                },
            )
            .await?;
        }
    }

    Ok(())
}
