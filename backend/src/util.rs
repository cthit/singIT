use serde::{de::Error, Deserialize};

/// A string that is validated to only contain the characters `[A-z0-9._-]`
pub struct PathSafeString(pub String);

impl<'de> Deserialize<'de> for PathSafeString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if s.chars()
            .all(|c: char| c.is_ascii_alphanumeric() || "_-.".contains(c))
        {
            Ok(PathSafeString(s))
        } else {
            Err(D::Error::custom("String contains forbiddden characters"))
        }
    }
}
