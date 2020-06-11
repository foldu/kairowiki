#[derive(Default)]
pub struct Builder<'a> {
    host_sources: Vec<&'a str>,
}

impl<'a> Builder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn host_source(mut self, source: &'a str) -> Self {
        self.host_sources.push(source);
        self
    }

    pub fn build(self) -> warp::reply::with::WithHeader {
        let header_value = format!("script-src {}", self.host_sources.join(" "));
        warp::reply::with::header("Content-Security-Policy", header_value)
    }
}
