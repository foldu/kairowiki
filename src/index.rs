use crate::article::ArticleTitle;
use std::{path::Path, time::Instant};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::{QueryParser, TermQuery},
    schema::{Field, IndexRecordOption, STORED, TEXT},
    IndexReader, IndexWriter, TantivyError, Term,
};
use tokio::sync::Mutex;

#[derive(Copy, Clone)]
pub struct Schema {
    pub title: Field,
    pub content: Field,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error from tantivy: {0}")]
    Tantivy(tantivy::TantivyError),

    #[error("Error while trying to index repo: {0}")]
    RepoIndex(#[from] std::io::Error),

    #[error("Can't rebuild from repo head: {0}")]
    Rebuild(crate::git::Error),
}

impl From<tantivy::TantivyError> for Error {
    fn from(other: TantivyError) -> Self {
        Error::Tantivy(other)
    }
}

pub struct Index {
    pub reader: IndexReader,
    pub writer: Mutex<IndexWriter>,
    pub schema: Schema,
}

impl Index {
    pub async fn open(
        index_path: impl AsRef<Path>,
        repo: &crate::git::read::ReadOnly<'_>,
    ) -> Result<Self, Error> {
        let (index, schema) = tokio::task::block_in_place(|| -> Result<_, Error> {
            let index_path = index_path.as_ref();

            std::fs::create_dir_all(index_path)?;
            let dir = MmapDirectory::open(index_path).map_err(TantivyError::from)?;
            let mut schema = tantivy::schema::Schema::builder();
            let title = schema.add_text_field("title", TEXT | STORED);
            let content = schema.add_text_field("content", TEXT | STORED);
            let schema = schema.build();

            let index = tantivy::Index::open_or_create(dir, schema.clone())?;
            let schema = Schema { title, content };
            Ok((index, schema))
        })?;

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommit)
            .try_into()?;
        let writer = index.writer(10 * (1 << 20))?;

        let ret = Index {
            schema,
            reader,
            writer: Mutex::new(writer),
        };

        ret.rebuild(repo).await?;

        Ok(ret)
    }

    pub async fn rebuild(&self, repo: &crate::git::read::ReadOnly<'_>) -> Result<(), Error> {
        let start_time = Instant::now();
        tracing::info!("Starting reindex");
        let mut writer = self.writer.lock().await;

        writer.delete_all_documents()?;

        repo.traverse_head_tree(|title, content| {
            let mut doc = tantivy::Document::new();
            doc.add_text(self.schema.title, &title);
            doc.add_text(self.schema.content, content);
            writer.add_document(doc);
        })
        .map_err(Error::Rebuild)?;

        writer.commit()?;

        tracing::info!("Reindexing completed in {:?}", Instant::now() - start_time);

        Ok(())
    }

    // TODO: use slow path fetch from repo when reindexing
    pub fn get_article(&self, title: &ArticleTitle) -> Option<String> {
        let searcher = self.reader.searcher();
        let term = Term::from_field_text(self.schema.title.clone(), title.as_ref());
        let term_query = TermQuery::new(term, IndexRecordOption::Basic);
        let results = searcher
            .search(&term_query, &TopDocs::with_limit(1))
            .unwrap();

        if results.is_empty() {
            None
        } else {
            let addr = results[0].1;
            let doc = searcher.doc(addr).unwrap();
            Some(
                doc.get_first(self.schema.content)
                    .unwrap()
                    .text()
                    .unwrap()
                    .to_owned(),
            )
        }
    }

    pub async fn update_article(&self, title: &ArticleTitle, content: &str) -> Result<(), Error> {
        let term = Term::from_field_text(self.schema.title, title.as_ref());
        let mut writer = self.writer.lock().await;
        writer.delete_term(term);

        let mut doc = tantivy::Document::new();
        doc.add_text(self.schema.title, title.as_ref());
        doc.add_text(self.schema.content, content.as_ref());
        writer.add_document(doc);

        writer.commit()?;

        Ok(())
    }

    pub fn search(&self, query: &str, ndocs: usize) -> Result<Vec<(String, String)>, Error> {
        let searcher = self.reader.searcher();

        let query = QueryParser::for_index(
            searcher.index(),
            vec![self.schema.title, self.schema.content],
        )
        .parse_query(query)
        // FIXME: decide what to do if query is malformed
        .unwrap();

        let results = searcher
            .search(&query, &TopDocs::with_limit(ndocs))
            .unwrap();

        // TODO: maybe add ellipsed contents of article?
        let mut found = Vec::with_capacity(ndocs);
        for (_score, addr) in results {
            let doc = searcher.doc(addr).unwrap();
            let title = doc
                .get_first(self.schema.title)
                .unwrap()
                .text()
                .unwrap()
                .to_owned();

            let content = doc.get_first(self.schema.content).unwrap().text().unwrap();

            let mut i = 200;
            let elided_content = loop {
                // I hope this uses is_char_boundary
                match content.get(0..i) {
                    Some(cont) => break cont,
                    None => {
                        i += 1;
                    }
                }
            };

            let elided_content = elided_content.to_owned();

            found.push((title, elided_content));
        }

        Ok(found)
    }
}

