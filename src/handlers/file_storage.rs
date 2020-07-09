use crate::{data::Data, relative_url::RelativeUrlOwned};
use bytes::Buf;
use futures_util::StreamExt;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not extract file field from request")]
    ExtractFileField,

    #[error("Could not upload file: {0}")]
    Upload(#[from] warp::Error),

    #[error("Could not store file: {0}")]
    Store(crate::file_storage::Error),
}

impl warp::reject::Reject for Error {}

async fn upload_(
    data: Data,
    mut form: warp::filters::multipart::FormData,
) -> Result<RelativeUrlOwned, Error> {
    let first_field = form.next().await.ok_or(Error::ExtractFileField)??;

    if first_field.name() != "file" {
        return Err(Error::ExtractFileField);
    }

    let mut file_stream = first_field.stream();
    let mut file = Vec::new();
    while let Some(buf) = file_stream.next().await {
        let buf = buf?;
        file.extend(buf.bytes());
    }

    data.file_storage.store(&file).await.map_err(Error::Store)
}

pub async fn upload(
    data: Data,
    _: crate::user_storage::UserAccount,
    form: warp::filters::multipart::FormData,
) -> Result<impl warp::Reply, warp::Rejection> {
    #[derive(serde::Serialize)]
    struct Reply<'a> {
        url: &'a str,
    }
    let url = upload_(data, form).await.map_err(warp::reject::custom)?;
    Ok(warp::reply::json(&Reply { url: url.as_ref() }))
}

