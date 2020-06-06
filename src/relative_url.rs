use bstr::ByteSlice;
use smallvec::SmallVec;
use std::{borrow::Cow, fmt::Write, io::Write as IOWrite};

#[derive(Debug)]
pub struct RelativeUrl<'a>(Cow<'a, str>);

pub struct Builder<'a> {
    path: Cow<'a, str>,
    query: SmallVec<[(&'a str, &'a str); 2]>,
}

#[derive(thiserror::Error, Debug)]
#[error("Empty url")]
pub struct Error;

impl<'a> Builder<'a> {
    pub fn element(mut self, elt: &str) -> Self {
        let path = self.path.to_mut();
        write!(path, "/{}", elt).unwrap();
        self
    }

    pub fn query(mut self, key: &'a str, value: &'a str) -> Self {
        self.query.push((key, value));
        self
    }

    pub fn build(self) -> RelativeUrl<'a> {
        if !self.query.is_empty() {
            let mut path = Vec::<u8>::from(self.path.into_owned());
            path.push(b'?');
            let mut iter = self.query.iter().peekable();
            while let Some((k, v)) = iter.next() {
                encode_into(k.as_bytes(), &mut path).unwrap();
                path.push(b'=');
                encode_into(v.as_bytes(), &mut path).unwrap();
                if let Some(_) = iter.peek() {
                    path.push(b'&');
                }
            }

            // unwrap can never fail, it's urlencoded
            // FIXME: maybe use unsafe String::from_utf8
            let ret = String::from_utf8(path).unwrap();

            RelativeUrl(Cow::Owned(ret))
        } else {
            RelativeUrl(self.path)
        }
    }
}

impl<'a> RelativeUrl<'a> {
    pub fn new(url: &'a str) -> Result<Self, Error> {
        if url.is_empty() {
            return Err(Error);
        }

        if url
            .as_bytes()
            .iter()
            .all(|&c| c == b'/' || !byte_needs_escaping(c))
        {
            Ok(Self(Cow::Borrowed(url)))
        } else {
            let mut ret = Vec::with_capacity(url.len());
            let url = url.as_bytes();
            let url = if url[0] == b'/' {
                ret.push(b'/');
                &url[1..]
            } else {
                url
            };
            let url = url.trim_end_with(|c| c == '/');

            let mut iter = url.split(|&b| b == b'/').peekable();
            while let Some(elt) = iter.next() {
                match elt {
                    b"" => (),
                    elt => {
                        encode_into(elt, &mut ret).unwrap();
                        if let Some(_) = iter.peek() {
                            ret.push(b'/');
                        }
                    }
                }
            }

            // unwrap ok, urlencoding something produces valid utf8
            Ok(RelativeUrl(Cow::Owned(String::from_utf8(ret).unwrap())))
        }
    }

    pub fn builder(base: &str) -> Result<Builder, Error> {
        Ok(Builder {
            path: RelativeUrl::new(base)?.0,
            query: SmallVec::new(),
        })
    }
}

impl<'a> AsRef<str> for RelativeUrl<'a> {
    fn as_ref(&self) -> &str {
        match self.0 {
            Cow::Owned(ref s) => s.as_str(),
            Cow::Borrowed(s) => s,
        }
    }
}

// adapted from https://github.com/bt/rust_urlencoding/blob/a86f1c49363d2edf19c4b656e42c706097aafa85/src/lib.rs#L18-L30
fn encode_into<W: IOWrite>(data: &[u8], mut escaped: W) -> Result<(), std::io::Error> {
    for byte in data.iter() {
        if !byte_needs_escaping(*byte) {
            escaped.write(std::slice::from_ref(byte))?;
        } else {
            escaped.write(&[b'%', to_hex_digit(*byte >> 4), to_hex_digit(*byte & 15)])?;
        }
    }
    Ok(())
}

fn byte_needs_escaping(b: u8) -> bool {
    match b {
        b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'.' | b'_' | b'~' => false,
        _ => true,
    }
}

fn to_hex_digit(digit: u8) -> u8 {
    match digit {
        0..=9 => b'0' + digit,
        10..=255 => b'A' - 10 + digit,
    }
}

// TODO: test Builder
#[cfg(test)]
mod test {
    use super::*;

    // TODO: more thorough testing
    #[test]
    fn relative_url_makes_sense() {
        assert_eq!(RelativeUrl::new("login").unwrap().0, Cow::Borrowed("login"));
        assert_eq!(
            RelativeUrl::new("/login").unwrap().0,
            Cow::Borrowed("/login")
        );
        assert_eq!(
            RelativeUrl::new("/logi n").unwrap().0,
            Cow::<str>::Owned("/logi%20n".to_string())
        );

        assert!(RelativeUrl::new("").is_err());

        assert_eq!(
            RelativeUrl::new("test/fish").unwrap().0,
            Cow::Borrowed("test/fish")
        );

        assert_eq!(
            RelativeUrl::new("/test 3///").unwrap().0,
            Cow::Borrowed("/test%203")
        );
    }
}

