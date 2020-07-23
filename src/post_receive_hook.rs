use anyhow::Context;
use std::{
    io::{stdin, BufRead},
    os::unix::net::UnixStream,
};

pub fn run() -> Result<(), anyhow::Error> {
    let mut ln = String::new();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    while stdin.read_line(&mut ln)? != 0 {
        let split = ln.split_whitespace().collect::<Vec<_>>();
        if let &[_parent_commit_id, _new_commit_id, "refs/heads/master"] = &split[..] {
            let _stream = UnixStream::connect("/tmp/test")
                .context("Can't connect to server, is it running?")?;
        }
    }

    Ok(())
}
