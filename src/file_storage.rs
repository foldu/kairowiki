use crate::{
    migrations::{MigrationInfo, NeedsMigration},
    relative_url::{RelativeUrl, RelativeUrlOwned},
};
use mime::Mime;
use std::str::FromStr;

pub struct FileStorage {
    config: Config,
    pool: sqlx::SqlitePool,
}

pub struct Config {
    pub storage_path: String,
    pub allowed_mime_types: Vec<Mime>,
    pub route: String,
}

impl MigrationInfo for FileStorage {
    fn migrations(&self) -> &'static [crate::migrations::Migration] {
        &[migration!("file_storage_schema")]
    }
}

impl FileStorage {
    pub async fn new(
        pool: sqlx::SqlitePool,
        config: Config,
    ) -> Result<NeedsMigration<Self>, std::io::Error> {
        // FIXME: error handling
        tokio::fs::create_dir_all(&config.storage_path)
            .await
            .unwrap();

        Ok(NeedsMigration::new(Self { config, pool }))
    }

    pub async fn store(&self, file: &[u8]) -> Result<RelativeUrlOwned, Error> {
        let mime = Mime::from_str(&tree_magic::from_u8(file))
            .expect("tree_magic returned invalid mime type");

        if !self.config.allowed_mime_types.contains(&mime) {
            return Err(Error::InvalidMime { mime });
        }

        let hash = tokio::task::block_in_place(|| blake3::hash(file));
        let mut cxn = self.pool.acquire().await.unwrap();

        let path = sqlx::query!(
            "SELECT relative_path FROM file_hash WHERE hash = ?",
            &hash.as_bytes()[..]
        )
        .fetch_optional(&mut *cxn)
        .await?
        .map(|row| row.relative_path);

        let path = match path {
            Some(path) => path,
            None => {
                let extension =
                    mime_extension(&mime).ok_or_else(|| Error::UnhandledMime { mime })?;
                // this is UNIX only so we can format (utf-8)paths
                let relative_path = format!("{}.{}", hash.to_hex(), extension);
                {
                    let target = format!("{}/{}", self.config.storage_path, relative_path);

                    tokio::fs::write(&target, file).await.unwrap();
                }

                sqlx::query!(
                    "INSERT INTO file_hash(relative_path, hash) VALUES (?, ?)",
                    relative_path,
                    &hash.as_bytes()[..]
                )
                .execute(&mut *cxn)
                .await?;

                relative_path
            }
        };

        Ok(RelativeUrl::builder(&self.config.route)
            .unwrap()
            .element(&path)
            .build()
            .owned())
    }
}

fn mime_extension(mime: &Mime) -> Option<&'static str> {
    if mime == &mime::IMAGE_PNG {
        Some("png")
    } else if mime == &mime::IMAGE_JPEG {
        Some("jpeg")
    } else if mime == &mime::IMAGE_GIF {
        Some("gif")
    } else if mime == &mime::IMAGE_SVG {
        Some("svg")
    } else {
        None
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Non allowed mime type: {}", mime)]
    InvalidMime { mime: mime::Mime },

    #[error("Unhandled mime: {}", mime)]
    UnhandledMime { mime: mime::Mime },

    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),
}
