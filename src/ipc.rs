use std::path::Path;
use tokio::{net::UnixListener, stream::Stream, sync::mpsc, task};

pub fn listen(sock_path: impl AsRef<Path>) -> Result<impl Stream<Item = ()>, std::io::Error> {
    let sock_path = sock_path.as_ref();
    // NOTE: race condition but unix is garbage anyway
    let _ = std::fs::remove_file(sock_path);
    let mut listener = UnixListener::bind(sock_path)?;

    let (mut tx, rx) = mpsc::channel(1);
    task::spawn(async move {
        loop {
            match listener.accept().await {
                Ok(_) => {
                    if let Err(_) = tx.send(()).await {
                        tracing::info!("Dropping unix listener");
                        return;
                    }
                }
                Err(e) => tracing::warn!("Dropped unix listener cxn: {}", e),
            }
        }
    });

    Ok(rx)
}
