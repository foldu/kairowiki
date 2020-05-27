#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("ajaj")]
    Io(std::io::Error),
}

impl warp::reject::Reject for Error {}
