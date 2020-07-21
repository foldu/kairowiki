use crate::article::{ArticleTitle, WikiArticle};
use parking_lot::Mutex;
use std::{path::Path, time::Instant};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::{QueryParser, TermQuery},
    schema::{Field, IndexRecordOption, STORED, STRING, TEXT},
    IndexReader, IndexWriter, TantivyError, Term,
};

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
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    schema: Schema,
}

impl Index {
    pub fn open(
        index_path: impl AsRef<Path>,
        repo: &crate::git::read::ReadOnly,
    ) -> Result<Self, Error> {
        let index_path = index_path.as_ref();

        std::fs::create_dir_all(index_path)?;
        let dir = MmapDirectory::open(index_path).map_err(TantivyError::from)?;
        let mut schema = tantivy::schema::Schema::builder();
        let title = schema.add_text_field("title", STRING | STORED);
        let content = schema.add_text_field("content", TEXT | STORED);
        let schema = schema.build();

        let index = tantivy::Index::open_or_create(dir, schema)?;
        let schema = Schema { title, content };

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

        ret.rebuild(repo)?;

        Ok(ret)
    }

    pub fn rebuild(&self, repo: &crate::git::read::ReadOnly) -> Result<(), Error> {
        let start_time = Instant::now();
        tracing::info!("Starting reindex");
        let mut writer = self.writer.lock();

        writer.delete_all_documents()?;

        repo.traverse_head_tree(|title, content| {
            let mut doc = tantivy::Document::new();
            doc.add_text(self.schema.title, &title);
            doc.add_text(self.schema.content, &content);
            writer.add_document(doc);
        })
        .map_err(Error::Rebuild)?;

        writer.commit()?;

        tracing::info!("Reindexing completed in {:?}", Instant::now() - start_time);

        Ok(())
    }

    pub fn get_article(
        &self,
        article: &WikiArticle,
        repo: &crate::git::Repo,
    ) -> Result<Option<String>, crate::git::Error> {
        // TODO/FIXME: this should only be done if the entire index is rebuilding, not if it's
        // just indexing a few docs. Does this even matter?
        if self.writer.is_locked() {
            let repo = repo.read()?;
            let head = repo.head()?.target().unwrap();
            return repo
                .article_at_rev(head, &article.path)
                .map(|ret| ret.map(|(_, cont)| cont));
        }

        let searcher = self.reader.searcher();
        let term = Term::from_field_text(self.schema.title, article.title.as_ref());
        let term_query = TermQuery::new(term, IndexRecordOption::Basic);
        let results = searcher
            .search(&term_query, &TopDocs::with_limit(1))
            .unwrap();

        if results.is_empty() {
            Ok(None)
        } else {
            let addr = results[0].1;
            let doc = searcher.doc(addr).unwrap();
            Ok(Some(
                doc.get_first(self.schema.content)
                    .unwrap()
                    .text()
                    .unwrap()
                    .to_owned(),
            ))
        }
    }

    pub fn update_article(&self, title: &ArticleTitle, content: &str) -> Result<(), Error> {
        let term = Term::from_field_text(self.schema.title, title.as_ref());
        let mut writer = self.writer.lock();
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

            // TODO: end on sentence boundary for preview instead of just cutting it off
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
