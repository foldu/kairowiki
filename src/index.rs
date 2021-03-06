use crate::article::{ArticleTitle, WikiArticle};
use parking_lot::Mutex;
use std::{path::Path, time::Instant};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::{QueryParser, TermQuery},
    schema::{Field, IndexRecordOption, TextFieldIndexing, TextOptions, STORED, STRING, TEXT},
    tokenizer::{LowerCaser, NgramTokenizer, TextAnalyzer},
    IndexReader, IndexWriter, SnippetGenerator, TantivyError, Term,
};

#[derive(Copy, Clone)]
pub struct Schema {
    pub title: Field,
    pub content: Field,
    pub title_ngrams: Field,
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

        let text_field_indexing = TextFieldIndexing::default()
            .set_tokenizer("title_ngrams")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_options = TextOptions::default().set_indexing_options(text_field_indexing);
        let title_ngrams = schema.add_text_field("title_ngrams", text_options);

        let schema = schema.build();

        let index = tantivy::Index::open_or_create(dir, schema)?;
        index.tokenizers().register(
            "title_ngrams",
            TextAnalyzer::from(NgramTokenizer::prefix_only(2, 3)).filter(LowerCaser),
        );

        let schema = Schema {
            title,
            content,
            title_ngrams,
        };

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

        let commit: Result<_, crate::git::Error> = try {
            let head = repo.head()?;
            let commit = head.peel_to_commit()?;
            commit
        };

        let commit = commit.unwrap();

        ret.rebuild(repo, &commit)?;

        Ok(ret)
    }

    fn create_doc(&self, title: &ArticleTitle, content: &str) -> tantivy::Document {
        let mut doc = tantivy::Document::new();
        doc.add_text(self.schema.title, title.as_ref());
        doc.add_text(self.schema.title_ngrams, title.as_ref());
        doc.add_text(self.schema.content, &content);
        doc
    }

    pub fn rebuild(
        &self,
        repo: &crate::git::read::ReadOnly,
        commit: &git2::Commit,
    ) -> Result<(), Error> {
        let start_time = Instant::now();
        tracing::info!("Starting reindex");
        let mut writer = self.writer.lock();

        writer.delete_all_documents()?;

        repo.traverse_commit_tree(commit, |title, content| {
            writer.add_document(self.create_doc(&title, &content));
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

    pub fn titles(&self) -> Vec<String> {
        let searcher = self.reader.searcher();
        let results = searcher
            .search(&tantivy::query::AllQuery, &TopDocs::with_limit(1000))
            .unwrap();

        let mut ret = Vec::with_capacity(results.len());
        for (_, addr) in results {
            let doc = searcher.doc(addr).unwrap();
            let title = doc
                .get_first(self.schema.title)
                .unwrap()
                .text()
                .unwrap()
                .to_string();
            ret.push(title);
        }

        ret
    }

    pub fn update_article(&self, title: &ArticleTitle, content: &str) -> Result<(), Error> {
        let term = Term::from_field_text(self.schema.title, title.as_ref());
        let mut writer = self.writer.lock();
        writer.delete_term(term);

        writer.add_document(self.create_doc(title, content));

        writer.commit()?;

        Ok(())
    }

    pub fn search(&self, query: &str, ndocs: usize) -> Result<Vec<SearchResult>, Error> {
        let searcher = self.reader.searcher();

        let query = QueryParser::for_index(
            searcher.index(),
            vec![self.schema.title_ngrams, self.schema.content],
        )
        .parse_query(query)
        // FIXME: decide what to do if query is malformed
        .unwrap();

        let results = searcher
            .search(&query, &TopDocs::with_limit(ndocs))
            .unwrap();

        let mut found = Vec::with_capacity(ndocs);
        // NOTE: skip allocation of SnippetGenerator when nothing found
        if results.is_empty() {
            return Ok(found);
        }

        let content_snippet_gen = SnippetGenerator::create(&searcher, &query, self.schema.content)?;

        for (_score, addr) in results {
            let doc = searcher.doc(addr)?;

            let title = doc
                .get_first(self.schema.title)
                .unwrap()
                .text()
                .unwrap()
                .to_string();
            let content = doc.get_first(self.schema.content).unwrap().text().unwrap();

            let content_text = match content_snippet_gen.snippet(content).to_html() {
                snippet if snippet.len() == 0 => {
                    if content.len() < 200 {
                        content.to_string()
                    } else {
                        let mut i = 200;
                        let elided_content = loop {
                            // I hope this uses is_char_boundary
                            match content.get(0..i) {
                                Some(cont) => break cont,
                                None => {
                                    i = usize::min(i + 1, content.len());
                                }
                            }
                        };
                        elided_content.to_string()
                    }
                }
                snippet => snippet,
            };

            found.push(SearchResult {
                title,
                content_text,
            });
        }

        Ok(found)
    }
}

pub struct SearchResult {
    pub title: String,
    pub content_text: String,
}
