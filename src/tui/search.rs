pub fn matches_query(haystack: &str, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }

    let haystack = haystack.to_ascii_lowercase();
    let mut chars = haystack.chars();
    for needle in query.to_ascii_lowercase().chars() {
        if !chars.any(|candidate| candidate == needle) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_matches_in_order() {
        assert!(matches_query(
            "node.node_modules /repo/node_modules",
            "nmod"
        ));
        assert!(!matches_query("rust.target", "zq"));
    }
}
