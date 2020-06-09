use crate::templates::HeadlineStart;
use askama::Template;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Tag};
use std::collections::VecDeque;
use syntect::{
    highlighting::{Theme, ThemeSet},
    html::highlighted_html_for_string,
    parsing::{Scope, SyntaxSet},
};

struct ParserWrap<'a> {
    it: pulldown_cmark::Parser<'a>,
    extra: VecDeque<Event<'a>>,
    renderer: &'a MarkdownRenderer,
}

pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        // TODO: make theme configurable
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get("InspiredGitHub")
            .map(|theme| theme.to_owned())
            .unwrap();

        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme,
        }
    }

    pub fn render(&self, markdown: &str) -> String {
        let mut rendered = String::new();
        let parser = ParserWrap {
            it: pulldown_cmark::Parser::new_ext(markdown, Options::all()),
            extra: VecDeque::new(),
            renderer: self,
        };
        pulldown_cmark::html::push_html(&mut rendered, parser);
        rendered
    }

    fn highlight(&self, s: &str, language: Option<&str>) -> String {
        let syntax = language
            .and_then(|lang| syntect::parsing::Scope::new(&format!("source.{}", lang)).ok())
            .and_then(|scope| self.syntax_set.find_syntax_by_scope(scope))
            .unwrap_or_else(|| {
                // `source` always exists in the default SyntaxSet
                let source_scope = Scope::new("source").unwrap();
                self.syntax_set.find_syntax_by_scope(source_scope).unwrap()
            });

        highlighted_html_for_string(s, &self.syntax_set, syntax, &self.theme)
    }
}

impl<'a> Iterator for ParserWrap<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(extra) = self.extra.pop_front() {
            return Some(extra);
        }

        let evt = self.it.next()?;
        match evt {
            Event::Start(Tag::Heading(n)) => {
                let n = std::cmp::max(std::cmp::min(n, 6), 1);
                // can generate head link
                match self.it.next() {
                    Some(Event::Text(headline)) => {
                        self.extra.push_back(Event::Text(headline.clone()));
                        while let Some(next) = self.it.next() {
                            match next {
                                Event::End(Tag::Heading(_)) => {
                                    self.extra
                                        .push_back(Event::Html(String::from("</a>").into()));
                                    self.extra.push_back(next);
                                    break;
                                }
                                next => self.extra.push_back(next),
                            }
                        }

                        Some(Event::Html(
                            HeadlineStart {
                                strength: n,
                                headline: headline.as_ref(),
                            }
                            .render()
                            .unwrap()
                            .into(),
                        ))
                    }
                    Some(other) => {
                        self.extra.push_front(other);
                        Some(evt)
                    }
                    None => Some(evt),
                }
            }
            Event::Start(Tag::CodeBlock(ref kind)) => {
                let text = self.it.next();
                let end = self.it.next();
                match (&text, &end) {
                    (Some(Event::Text(s)), Some(Event::End(Tag::CodeBlock(_)))) => {
                        let lang = match kind {
                            CodeBlockKind::Fenced(lang) => Some(lang.as_ref()),
                            CodeBlockKind::Indented => None,
                        };

                        Some(Event::Html(CowStr::from(self.renderer.highlight(s, lang))))
                    }
                    // this probably can't happen but if it happens just put it back
                    _ => {
                        if let Some(text) = text {
                            self.extra.push_front(text);
                        }
                        if let Some(end) = end {
                            self.extra.push_front(end);
                        }
                        Some(evt)
                    }
                }
            }
            Event::Code(code) => Some(Event::Html(CowStr::from(
                self.renderer.highlight(&code, None),
            ))),
            _ => Some(evt),
        }
    }
}
