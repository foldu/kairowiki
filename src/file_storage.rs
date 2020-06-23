use crate::{
    migrations::{MigrationInfo, NeedsMigration},
    relative_url::{RelativeUrl, RelativeUrlOwned},
};
use mime::Mime;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    str::FromStr,
};

pub struct FileStorage {
    storage_path: String,
    allowed_mime_types: BTreeMap<Mime, String>,
    route: String,
    pool: sqlx::SqlitePool,
}

pub struct Config<'a> {
    pub storage_path: String,
    pub allowed_mime_types: Vec<Mime>,
    pub route: String,
    pub mime_types_path: &'a Path,
}

impl MigrationInfo for FileStorage {
    fn migrations(&self) -> &'static [crate::migrations::Migration] {
        &[migration!("file_storage_schema")]
    }
}

impl FileStorage {
    pub fn new(pool: sqlx::SqlitePool, config: Config) -> Result<NeedsMigration<Self>, Error> {
        std::fs::create_dir_all(&config.storage_path)?;

        let allowed_mime_types =
            find_mime_extensions(&config.mime_types_path, &config.allowed_mime_types)?;

        Ok(NeedsMigration::new(Self {
            allowed_mime_types,
            route: config.route,
            storage_path: config.storage_path,
            pool,
        }))
    }

    pub async fn store(&self, file: &[u8]) -> Result<RelativeUrlOwned, Error> {
        let mime = Mime::from_str(&tree_magic::from_u8(file))
            .expect("tree_magic returned invalid mime type");

        let ext = if let Some(ext) = self.allowed_mime_types.get(&mime) {
            ext
        } else {
            return Err(Error::InvalidMime { mime });
        };

        let hash = tokio::task::block_in_place(|| blake3::hash(file));
        let mut cxn = self.pool.acquire().await.unwrap();

        let path = sqlx::query!(
            "SELECT relative_path FROM file_hash WHERE hash = ?",
            &hash.as_bytes()[..]
        )
        .fetch_optional(&mut *cxn)
        .await?
        .map(|row| row.relative_path);

        // maybe TODO: check if path exists and allow reupload
        let path = match path {
            Some(path) => path,
            None => {
                // this is UNIX only so we can format (utf-8)paths
                let relative_path = format!("{}.{}", hash.to_hex(), ext);
                let target = format!("{}/{}", self.storage_path, relative_path);

                // 'locks' the file upload
                // TODO: do not return Error when this is already inserted, just pretend the file
                // is already uploaded
                sqlx::query!(
                    "INSERT INTO file_hash(relative_path, hash) VALUES (?, ?)",
                    relative_path,
                    &hash.as_bytes()[..]
                )
                .execute(&mut *cxn)
                .await?;

                if let Err(e) = tokio::fs::write(&target, file).await {
                    sqlx::query!("DELETE FROM file_hash WHERE hash = ?", &hash.as_bytes()[..])
                        .execute(&mut *cxn)
                        .await?;

                    return Err(Error::Io(e));
                }

                relative_path
            }
        };

        Ok(RelativeUrl::builder(&self.route)
            .unwrap()
            .element(&path)
            .build()
            .owned())
    }
}

fn find_mime_extensions(
    path: &Path,
    mimes: &[Mime],
) -> Result<std::collections::BTreeMap<Mime, String>, Error> {
    let mut reader = File::open(path)
        .map(BufReader::new)
        .map_err(Error::CantReadMimeTypes)?;
    let mut ln = String::new();
    // NOTE: BTreeMap has no with_capacity
    let mut ret = BTreeMap::new();
    let mut visited = vec![false; mimes.len()];

    while reader
        .read_line(&mut ln)
        .map_err(Error::CantReadMimeTypes)?
        != 0
    {
        let mut it = ln.split_whitespace();
        let mime_pos = it
            .next()
            .and_then(|mime| Mime::from_str(mime).ok())
            .and_then(|mime| mimes.iter().position(|m| m == &mime));

        if let Some(pos) = mime_pos {
            if let Some(last_extension) = it.last() {
                visited[pos] = true;
                ret.insert(mimes[pos].clone(), last_extension.into());
            }
        }

        ln.clear();
    }

    let not_found = mimes
        .iter()
        .zip(visited)
        .find(|(_, visited)| !visited)
        .map(|(mime, _)| mime);

    if let Some(not_found) = not_found {
        Err(Error::MimeDoesNotExist {
            mime: not_found.clone(),
        })
    } else {
        Ok(ret)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Non allowed mime type: {}", mime)]
    InvalidMime { mime: mime::Mime },

    #[error("Mime {mime} does not exist")]
    MimeDoesNotExist { mime: Mime },

    #[error("Could not read mime.types: {0}")]
    CantReadMimeTypes(std::io::Error),

    #[error("IO error on storage: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),
}

#[test]
fn can_find_mime_extensions() {
    let mime_path = std::env::var_os("MIME_TYPES_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate::data::default_mime_types_path());
    find_mime_extensions(&mime_path, &[mime::IMAGE_JPEG]).unwrap();
}
