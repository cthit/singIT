use std::cmp::{Ord, Ordering, PartialOrd};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FuzzyScore {
    pub score: i32,
    pub matches: Vec<FuzzyCharMatch>,
}

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct FuzzyCharMatch {
    pub base_str_index: usize,
    pub search_str_index: usize,
}

impl PartialOrd for FuzzyScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FuzzyScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

/// Compare a base string to a user-input search
///
/// Returns a tuple of the match score, as well as the indices of every char in `search` which maps
/// to an index in `base`
pub fn compare<B, S>(base: B, search: S) -> FuzzyScore
where
    B: Iterator<Item = char> + Clone,
    S: IntoIterator<Item = char>,
{
    let mut base = base.into_iter().enumerate();

    // How alike the search string is to self.name
    //let mut score = -(search.len() as i32);
    let mut score = 0;

    // Vector of which char index in s maps to which char index in self.name
    let mut matches = vec![];

    for (i, sc) in search.into_iter().enumerate() {
        let sc = sc.to_ascii_lowercase();
        let mut add = 3;
        let mut base_tmp = base.clone();
        while let Some((j, bc)) = base_tmp.next() {
            let bc = bc.to_ascii_lowercase();
            if bc == sc {
                matches.push(FuzzyCharMatch {
                    search_str_index: i,
                    base_str_index: j,
                });

                score += add;
                base = base_tmp;
                break;
            } else {
                add = 2;
            }
        }
    }

    FuzzyScore { score, matches }
}
