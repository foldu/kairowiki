use std::str::CharIndices;
use tantivy::tokenizer::{BoxTokenStream, Token, TokenStream, Tokenizer};

#[derive(Clone)]
pub struct TitleSegmentsTokenizer;

impl Tokenizer for TitleSegmentsTokenizer {
    fn token_stream<'a>(&self, text: &'a str) -> BoxTokenStream<'a> {
        println!("start");
        BoxTokenStream::from(TitleSegmentsTokenStream {
            text,
            chars: text.char_indices(),
            token: Token::default(),
        })
    }
}

struct TitleSegmentsTokenStream<'a> {
    text: &'a str,
    chars: CharIndices<'a>,
    token: Token,
}

impl<'a> TitleSegmentsTokenStream<'a> {
    fn search_token_end(&mut self) -> usize {
        (&mut self.chars)
            .find(|&(_, c)| c == '/')
            .map(|(offset, _)| offset)
            .unwrap_or_else(|| self.text.len())
    }
}

impl<'a> TokenStream for TitleSegmentsTokenStream<'a> {
    fn advance(&mut self) -> bool {
        self.token.text.clear();
        self.token.position = self.token.position.wrapping_add(1);
        while let Some((offset_from, c)) = self.chars.next() {
            if c != '/' {
                let offset_to = self.search_token_end();
                self.token.offset_from = offset_from;
                self.token.offset_to = offset_to;
                println!("{}", &self.text[offset_from..offset_to]);
                self.token.text.push_str(&self.text[offset_from..offset_to]);
                return true;
            } else {
                println!("AAAAAAAAA {}", &self.text[offset_from..]);
            }
        }
        false
    }

    fn token(&self) -> &Token {
        &self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.token
    }
}
