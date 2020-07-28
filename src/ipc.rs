use crate::serde::Oid;
use bytes::Bytes;
use futures_util::SinkExt;
use std::{path::Path, time::Duration};
use tokio::{
    net::{UnixListener, UnixStream},
    stream::{Stream, StreamExt},
    sync::mpsc::{self, Sender},
    task,
    time::timeout,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub const SOCK_PATH: &str = "/tmp/.kairowiki";

pub async fn send_update(
    sock_path: impl AsRef<Path>,
    update: &Update,
) -> Result<(), std::io::Error> {
    let sock = UnixStream::connect(sock_path).await?;
    let mut framed = frame(sock);
    let msg = Bytes::from(serde_json::to_vec(update).expect("Update is not json encodable"));
    framed.send(msg).await?;
    framed.flush().await?;
    Ok(())
}

pub fn listen(sock_path: impl AsRef<Path>) -> Result<impl Stream<Item = Update>, std::io::Error> {
    let sock_path = sock_path.as_ref();
    // NOTE: race condition but unix is garbage anyway
    let _ = std::fs::remove_file(sock_path);
    let listener = UnixListener::bind(sock_path)?;

    let (tx, rx) = mpsc::channel(1);
    task::spawn(listen_task(listener, tx));

    Ok(rx)
}

#[tracing::instrument]
async fn listen_task(mut listener: UnixListener, mut tx: Sender<Update>) {
    loop {
        // NOTE: not spawning a new task here because reloading is inherently serial
        match listener.accept().await {
            Ok((sock, _addr)) => {
                let mut framed = frame(sock);
                match timeout(Duration::from_millis(500), framed.next()).await {
                    Ok(Some(Ok(msg))) => match serde_json::from_slice::<Update>(&msg[..]) {
                        Ok(update) => {
                            if let Err(_) = tx.send(update).await {
                                return;
                            }
                        }
                        Err(e) => tracing::warn!("Client send invalid message: {}", e),
                    },
                    Ok(Some(Err(e))) => tracing::warn!("Client errored out: {}", e),
                    Ok(None) => tracing::warn!("Client send no bytes"),
                    Err(_) => tracing::warn!("Client timed out"),
                }
            }
            Err(e) => tracing::warn!("Ipc listener accept error: {}", e),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Update {
    pub parent_commit_id: Oid,
    pub new_commit_id: Oid,
}

fn frame(sock: UnixStream) -> Framed<UnixStream, LengthDelimitedCodec> {
    Framed::new(sock, LengthDelimitedCodec::new())
}
