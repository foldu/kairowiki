// will probably need to write my own html emitter to customize output

pub fn render(src: &str) -> String {
    let mut rendered = String::new();
    let parser = pulldown_cmark::Parser::new_ext(src, pulldown_cmark::Options::ENABLE_TABLES);
    pulldown_cmark::html::push_html(&mut rendered, parser);
    rendered
}
