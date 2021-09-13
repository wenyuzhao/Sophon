use alloc::{borrow::ToOwned, format, string::String};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct UriSyntaxError;

/// authority = [userinfo@]host[:port]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Authority<'a> {
    pub userinfo: Option<&'a str>,
    pub host: &'a str,
    pub port: Option<&'a str>,
}

impl<'a> Authority<'a> {
    pub fn new(s: &'a str) -> Result<Self, UriSyntaxError> {
        let (userinfo, s) = match s.find('@') {
            Some(i) => {
                let (u, s) = s.split_at(i);
                (Some(u), s.strip_prefix('@').unwrap())
            }
            _ => (None, s),
        };
        let (host, port) = match s.find(':') {
            Some(i) => {
                let (u, s) = s.split_at(i);
                (u, Some(s.strip_prefix(':').unwrap()))
            }
            _ => (s, None),
        };
        Ok(Self {
            userinfo,
            host,
            port,
        })
    }

    pub fn to_string(&self) -> String {
        format!(
            "{}{}{}",
            self.userinfo
                .map(|x| format!("{}@", x))
                .unwrap_or_else(|| "".to_owned()),
            self.host,
            self.port
                .map(|x| format!(":{}", x))
                .unwrap_or_else(|| "".to_owned()),
        )
    }
}

/// URI = scheme:[//authority]path[?query][#fragment]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Uri<'s> {
    pub raw: &'s str,
    pub scheme: &'s str,
    pub authority: Option<Authority<'s>>,
    pub path: &'s str,
    pub query: Option<&'s str>,
    pub fragment: Option<&'s str>,
}

impl<'s> Uri<'s> {
    pub fn new(raw: &'s str) -> Result<Self, UriSyntaxError> {
        let s = raw;
        let (scheme, s) = s.split_at(s.find(':').ok_or(UriSyntaxError)?);
        let (_, s) = s.split_at(1);
        let (authority, s) = match s.strip_prefix("//") {
            Some(s) => {
                let (a, s) = match s.find(|c: char| c == '/' || c == '?' || c == '#') {
                    Some(i) => s.split_at(i),
                    _ => (s, ""),
                };
                (Some(Authority::new(a)?), s)
            }
            None => (None, s),
        };
        let (path, s) = if !s.starts_with("/") {
            ("", s)
        } else {
            s.split_at(
                s.find(|c: char| c == '?' || c == '#')
                    .unwrap_or_else(|| s.len()),
            )
        };
        let (query, s) = match s.strip_prefix("?") {
            Some(s) => {
                let (q, s) = s.split_at(s.find('#').unwrap_or_else(|| s.len()));
                (Some(q), s)
            }
            None => (None, s),
        };
        let (fragment, s) = match s.strip_prefix("#") {
            Some(s) => (Some(s), ""),
            None => (None, s),
        };
        if !s.is_empty() {
            return Err(UriSyntaxError);
        }

        Ok(Self {
            raw,
            scheme,
            authority,
            path,
            query,
            fragment,
        })
    }
}

impl<'s> Uri<'s> {
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl<'s> AsRef<str> for Uri<'s> {
    fn as_ref(&self) -> &str {
        self.raw.as_ref()
    }
}

pub trait AsUri {
    fn as_uri(&self) -> Uri<'_>;
    fn as_str(&self) -> &str {
        self.as_uri().raw.as_ref()
    }
}

impl<T: AsRef<str>> AsUri for T {
    fn as_uri(&self) -> Uri<'_> {
        Uri::new(self.as_ref()).unwrap()
    }
}
