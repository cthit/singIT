use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use std::ops::Not;

#[derive(Default)]
pub struct ParsedQuery<'a> {
    /// Unspecified query.
    pub plain: Option<Cow<'a, str>>,

    /// Query a specific title
    pub title: Option<&'a str>,

    /// Query a specific artist
    pub artist: Option<&'a str>,

    /// Whether the song is a duet
    pub duet: Option<bool>,

    /// Whether the song has a video
    pub video: Option<bool>,

    /// Query a specific language
    pub language: Option<&'a str>,

    /// Query a specific genre
    pub genre: Option<&'a str>,

    /// Query from a specifc year
    pub year: Option<&'a str>,
}

impl<'a> ParsedQuery<'a> {
    pub fn parse(s: &'a str) -> Self {
        let mut parsed = ParsedQuery {
            plain: extract_plain(s),
            ..Default::default()
        };

        let kvs = extract_key_values(s);

        for (k, v) in kvs {
            match k {
                "title" => parsed.title = Some(v),
                "artist" => parsed.artist = Some(v),
                "duet" => parsed.duet = parse_bool(v),
                "video" => parsed.video = parse_bool(v),
                "lang" => parsed.language = Some(v),
                "genre" => parsed.genre = Some(v),
                "year" => parsed.year = Some(v),
                _ => {}
            }
        }

        parsed
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "true" | "yes" => Some(true),
        "false" | "no" => Some(false),
        _ => None,
    }
}

fn extract_plain(s: &str) -> Option<Cow<str>> {
    let plain: String =
        s.split(' ')
            .filter(|word| !word.contains(':'))
            .fold(String::new(), |mut a, b| {
                if !a.is_empty() {
                    a.push(' ');
                }
                a.push_str(b);
                a
            });

    plain.is_empty().not().then(|| Cow::Owned(plain))
}

fn extract_key_values(s: &str) -> impl Iterator<Item = (&str, &str)> {
    s.split_whitespace().filter_map(|s| s.split_once(':'))
}

impl Display for ParsedQuery<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let mut w = |prefix: &str, display: Option<&dyn Display>| -> fmt::Result {
            match display {
                Some(display) => {
                    if first {
                        first = false;
                    } else {
                        write!(f, " ")?;
                    }
                    write!(f, "{}{}", prefix, display)
                }
                None => Ok(()),
            }
        };

        fn display<T: Display>(v: &Option<T>) -> Option<&dyn Display> {
            v.as_ref().map(|s| s as &dyn Display)
        }

        w("", display(&self.plain))?;
        w("title:", display(&self.title))?;
        w("artist:", display(&self.artist))?;
        w("duet:", display(&self.duet))?;
        w("video:", display(&self.video))?;
        w("lang:", display(&self.language))?;
        w("genre:", display(&self.genre))?;
        w("year:", display(&self.year))?;

        Ok(())
    }
}
