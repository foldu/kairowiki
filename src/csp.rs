#![allow(unused)]
use itertools::Itertools;

#[derive(Default)]
pub struct Builder<'a> {
    script_sources: Vec<&'a str>,
    style_sources: Vec<&'a str>,
    worker_sources: Vec<&'a str>,
}

fn create_directives(directives: &[(&str, &[&str])]) -> String {
    directives
        .iter()
        .filter_map(|(name, sources)| {
            if sources.is_empty() {
                None
            } else {
                Some(format!("{} {};", name, sources.join(" ")))
            }
        })
        .join(" ")
}

impl<'a> Builder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn style_source(mut self, source: &'a str) -> Self {
        self.style_sources.push(source);
        self
    }

    pub fn style_sources(mut self, sources: impl IntoIterator<Item = &'a str>) -> Self {
        self.style_sources.extend(sources);
        self
    }

    pub fn script_sources(mut self, sources: impl IntoIterator<Item = &'a str>) -> Self {
        self.script_sources.extend(sources);
        self
    }

    pub fn worker_source(mut self, source: &'a str) -> Self {
        self.worker_sources.push(source);
        self
    }

    pub fn build(self) -> warp::reply::with::WithHeader {
        warp::reply::with::header(
            "Content-Security-Policy",
            create_directives(&[
                ("script-src", &self.script_sources),
                ("style-src", &self.style_sources),
                ("worker-src", &self.worker_sources),
            ]),
        )
    }
}
