use warp::http::{HeaderMap, HeaderValue};

#[derive(Default)]
pub struct Builder<'a> {
    script_sources: Vec<&'a str>,
    style_sources: Vec<&'a str>,
    worker_sources: Vec<&'a str>,
}

fn create_directives(directives: &[(Vec<&str>, &str)]) -> String {
    directives
        .into_iter()
        .filter_map(|(sources, name)| {
            if sources.is_empty() {
                None
            } else {
                Some(format!("{} {};", name, sources.join(" ")))
            }
        })
        .collect::<Vec<_>>()
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
                (self.script_sources, "script-src"),
                (self.style_sources, "style-src"),
                (self.worker_sources, "worker-src"),
            ]),
        )
    }
}
