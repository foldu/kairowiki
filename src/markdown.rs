use crate::templates::HeadlineStart;
use askama::Template;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Tag};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};
use syntect::{
    highlighting::ThemeSet,
    html::{css_for_theme, ClassedHTMLGenerator},
    parsing::{Scope, SyntaxSet},
};

struct ParserWrap<'a, It> {
    it: It,
    extra: VecDeque<Event<'a>>,
    renderer: &'a MarkdownRenderer,
}

pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
}

fn title_to_id(title: &str) -> String {
    let prefix = "u-";
    let mut ret = String::with_capacity(title.len() + prefix.len());
    ret.push_str(prefix);
    ret.extend(title.chars().map(|c| match c {
        ' ' => '-',
        c => c,
    }));
    ret
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unknown theme: {}, allowed: {}", theme_name, theme_list)]
    ThemeNotFound {
        theme_name: String,
        theme_list: String,
    },

    #[error("Can't write theme to {}: {}", path.display(), source)]
    WriteTheme {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl MarkdownRenderer {
    pub fn new(theme_name: &str, theme_path: impl AsRef<Path>) -> Result<Self, Error> {
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get(theme_name)
            .map(|theme| theme.to_owned())
            .ok_or_else(|| {
                let theme_list = theme_set
                    .themes
                    .keys()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");

                Error::ThemeNotFound {
                    theme_name: theme_name.into(),
                    theme_list,
                }
            })?;

        let theme_path = theme_path.as_ref();
        std::fs::write(theme_path, css_for_theme(&theme).as_bytes()).map_err(|source| {
            Error::WriteTheme {
                path: theme_path.to_owned(),
                source,
            }
        })?;

        Ok(Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
        })
    }

    pub fn render(&self, markdown: &str) -> String {
        let mut rendered = String::new();
        let parser =
            pulldown_cmark::Parser::new_ext(markdown, Options::all()).filter_map(
                |node| match node {
                    Event::Html(_) => None,
                    other => Some(other),
                },
            );
        let parser = ParserWrap {
            it: parser,
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

        let mut gen = ClassedHTMLGenerator::new(&syntax, &self.syntax_set);
        for ln in s.lines() {
            gen.parse_html_for_line(ln);
        }
        gen.finalize()
    }

    fn highlight_block(&self, s: &str, language: Option<&str>) -> String {
        format!("<pre>{}</pre>", self.highlight(s, language))
    }
}

impl<'a, It> Iterator for ParserWrap<'a, It>
where
    It: Iterator<Item = Event<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(extra) = self.extra.pop_front() {
            return Some(extra);
        }

        let evt = self.it.next()?;
        // NOTE: self.extra is empty here
        match evt {
            Event::Start(Tag::Heading(n)) => {
                let n = std::cmp::max(std::cmp::min(n, 6), 1);
                match self.it.next() {
                    // it has text so we can create headline
                    Some(Event::Text(headline)) => {
                        // defer all other events
                        self.extra.push_back(Event::Text(headline.clone()));
                        // consume input until we find the headline closing
                        while let Some(next) = self.it.next() {
                            match next {
                                Event::End(Tag::Heading(_)) => {
                                    // close opened link tag
                                    self.extra
                                        .push_back(Event::Html(String::from("</a>").into()));
                                    self.extra.push_back(next);
                                    break;
                                }
                                next => self.extra.push_back(next),
                            }
                        }

                        // open link tag
                        Some(Event::Html(
                            HeadlineStart {
                                strength: n,
                                headline: headline.as_ref(),
                                id: &title_to_id(&headline),
                            }
                            .render()
                            .unwrap()
                            .into(),
                        ))
                    }
                    // can't get a title from this
                    Some(other) => {
                        // put it back
                        self.extra.push_back(other);
                        Some(evt)
                    }
                    // empty headline
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

                        Some(Event::Html(CowStr::from(
                            self.renderer.highlight_block(s, lang),
                        )))
                    }
                    // this probably can't happen but if it happens just put it back
                    _ => {
                        if let Some(text) = text {
                            self.extra.push_back(text);
                        }
                        if let Some(end) = end {
                            self.extra.push_back(end);
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
