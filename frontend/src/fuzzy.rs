pub type FuzzyScore = i32;

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

    for (_i, sc) in search.into_iter().enumerate() {
        let sc = sc.to_ascii_lowercase();
        let mut add = 3;
        let mut base_tmp = base.clone();
        while let Some((_j, bc)) = base_tmp.next() {
            let bc = bc.to_ascii_lowercase();
            if bc == sc {
                score += add;
                base = base_tmp;
                break;
            } else {
                add = 2;
            }
        }
    }

    score
}

pub fn max_score(query: &str) -> FuzzyScore {
    compare(query.chars(), query.chars())
}
