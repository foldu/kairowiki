use crate::article::ArticleTitle;
use std::{ffi::OsStr, path::Path, time::Instant};
use tantivy::{
    directory::MmapDirectory,
    doc,
    schema::{Field, STORED, TEXT},
    Index, IndexReader, IndexWriter, TantivyError, Term,
};

pub struct Schema {
    pub title: Field,
    pub content: Field,
}

pub fn open(index_path: impl AsRef<Path>) -> Result<(Schema, IndexReader, IndexWriter), Error> {
    let index_path = index_path.as_ref();
    std::fs::create_dir_all(index_path)?;
    let dir = MmapDirectory::open(index_path).map_err(TantivyError::from)?;
    let mut schema = tantivy::schema::Schema::builder();
    let title = schema.add_text_field("title", TEXT | STORED);
    let content = schema.add_text_field("content", TEXT | STORED);
    let schema = schema.build();

    let index = Index::open_or_create(dir, schema.clone())?;
    let schema = Schema { title, content };

    let reader = index
        .reader_builder()
        .reload_policy(tantivy::ReloadPolicy::OnCommit)
        .try_into()?;
    let writer = index.writer(10 * (1 << 20))?;

    Ok((schema, reader, writer))
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error from tantivy: {0}")]
    Tantivy(tantivy::TantivyError),

    #[error("Error while trying to index repo: {0}")]
    RepoIndex(#[from] std::io::Error),
}

impl From<tantivy::TantivyError> for Error {
    fn from(other: TantivyError) -> Self {
        Error::Tantivy(other)
    }
}

// TODO: handle inconsistent indexes
pub fn rebuild(
    repo: impl AsRef<Path>,
    schema: &Schema,
    writer: &mut IndexWriter,
) -> Result<(), Error> {
    let start_time = Instant::now();
    tracing::info!("Starting reindex");

    writer.delete_all_documents()?;

    let repo = repo.as_ref();
    let entries = walkdir::WalkDir::new(repo).into_iter().filter_entry(|ent| {
        !ent.file_name()
            .to_str()
            .map(|s| s.starts_with("."))
            .unwrap_or(false)
    });

    for entry in entries {
        // not following symbolic links and no max depth is given so the only
        // way this can fail is io
        let entry = entry.map_err(|e| e.into_io_error().unwrap())?;
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .map(|ext| OsStr::new("md") == ext)
                .unwrap_or(false)
        {
            // just ignore invalid titles
            if let Ok(title) = ArticleTitle::from_path(repo, entry.path()) {
                tracing::info!("Indexed {}", title);
                let content = std::fs::read_to_string(entry.path())?;
                let title = title.as_ref();
                writer.add_document(doc!(
                schema.title => title.as_str(),
                schema.content => content,
                ));
            }
        }
    }

    writer.commit()?;

    tracing::info!("Reindexing completed in {:?}", Instant::now() - start_time);

    Ok(())
}
pub fn update_article(
    schema: &Schema,
    writer: &mut IndexWriter,
    title: &ArticleTitle,
    content: &str,
) -> Result<(), Error> {
    let term = Term::from_field_text(schema.title.clone(), title.as_ref());
    writer.delete_term(term);

    let mut doc = tantivy::Document::new();
    doc.add_text(schema.title, title.as_ref());
    doc.add_text(schema.content, content.as_ref());
    writer.add_document(doc);

    writer.commit()?;

    Ok(())
}

