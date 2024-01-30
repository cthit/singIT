use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Category {
    pub title: String,
}
