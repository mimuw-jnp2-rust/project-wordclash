use std::collections::HashMap;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchLetter {
    Null = 0, // not present in word
    Close = 1, // present elsewhere
    Exact = 2, // present here
}

pub fn match_word(base: &str, provided: &str) -> Vec<MatchLetter> {
    let mut output = Vec::with_capacity(base.len());
    output.resize(base.len(), MatchLetter::Null);

    if provided.len() != base.len() {
        return output; // Mismatched lengths
    }
    
    let inexact: Vec<(usize, (char, char))> = base.chars()
        .zip(provided.chars())
        .enumerate()
        .filter(|(i, (a, b))| {
            if a == b {
                // exact match
                output[*i] = MatchLetter::Exact;
            }
            a != b
        })
        .collect();
    // Splitting inexact matches
    let mut inexact_base: HashMap<char, usize> = HashMap::new();
    inexact.iter()
        .for_each(|(_, (a, _))| {
            *inexact_base.entry(*a).or_insert(0) += 1;
        });
    
    inexact.iter()
        .for_each(|(i, (_, b))| {
            if let Some(v) = inexact_base.get_mut(b) {
                output[*i] = MatchLetter::Close;
                *v -= 1;
                if *v == 0 {
                    inexact_base.remove(b);
                }
            }
        });
    output
}

#[cfg(test)]
mod test {
    use super::*;
    use MatchLetter::*;

    #[test]
    fn test_basic_matching() {
        let base = "slide";

        let wmatch = match_word(base, "tower");
        assert_eq!(wmatch, vec![Null,  Null,  Null,  Close, Null]);
        let wmatch = match_word(base, "lease");
        assert_eq!(wmatch, vec![Close, Null,  Null,  Close, Exact]);
        let wmatch = match_word(base, "slide");
        assert_eq!(wmatch, vec![Exact, Exact, Exact, Exact, Exact]);
    }
}