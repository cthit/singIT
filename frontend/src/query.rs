use crate::song::Song;
use rand::seq::SliceRandom;
use rand::Rng;
use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use std::ops::Not;

#[derive(Default)]
pub struct ParsedQuery<'a> {
    /// Unspecified query.
    pub plain: Option<Cow<'a, str>>,

    /// Query a specific title
    pub title: Option<Cow<'a, str>>,

    /// Query a specific artist
    pub artist: Option<Cow<'a, str>>,

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

    /// Query songs from the specified custom list.
    pub list: Option<&'a str>,
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
                "title" => parsed.title = Some(Cow::Borrowed(v)),
                "artist" => parsed.artist = Some(Cow::Borrowed(v)),
                "duet" => parsed.duet = parse_bool(v),
                "video" => parsed.video = parse_bool(v),
                "lang" => parsed.language = Some(v),
                "genre" => parsed.genre = Some(v),
                "year" => parsed.year = Some(v),
                "list" => parsed.list = Some(v),
                _ => {}
            }
        }

        parsed
    }

    /// Generate a parsed query with a few random fields matching a song
    pub fn random<R: Rng>(song: &'a Song, rng: &mut R) -> Self {
        let until_space = |s: &'a str| -> &'a str { s.split_whitespace().next().unwrap_or("") };

        let join_spaces = |s: &'a str| -> Cow<'a, str> {
            let s = s.trim();
            if s.contains(char::is_whitespace) {
                s.replace(char::is_whitespace, "").into()
            } else {
                Cow::Borrowed(s)
            }
        };

        let mut primary_fields: [&dyn Fn(Self) -> Self; 4] = [
            &|query| Self {
                plain: Some(Cow::Borrowed(&song.title)),
                ..query
            },
            &|query| Self {
                plain: Some(Cow::Borrowed(&song.artist)),
                ..query
            },
            &|query| Self {
                title: Some(join_spaces(&song.title)),
                ..query
            },
            &|query| Self {
                artist: Some(join_spaces(&song.artist)),
                ..query
            },
        ];

        let mut extra_fields: [&dyn Fn(Self) -> Self; 3] = [
            &|query| Self {
                language: song.language.as_deref().map(until_space),
                ..query
            },
            &|query| Self {
                genre: song.genre.as_deref().map(until_space),
                ..query
            },
            &|query| Self {
                year: song.year.as_deref().map(until_space),
                ..query
            },
        ];

        primary_fields.shuffle(rng);
        extra_fields.shuffle(rng);

        let primary_fields = primary_fields.into_iter().take(1);
        let extra_fields = extra_fields.into_iter().take(rng.gen_range(0..2));

        primary_fields
            .chain(extra_fields)
            .fold(Self::default(), |query, field| field(query))
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "true" | "yes" | "y" => Some(true),
        "false" | "no" | "n" => Some(false),
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

    plain.is_empty().not().then_some(Cow::Owned(plain))
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
        w("list:", display(&self.list))?;

        Ok(())
    }
}
