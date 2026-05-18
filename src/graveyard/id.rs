use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Monotonic-ish, lexicographically sortable grave id.
///
/// Format: 12-hex-char unix-millis prefix + 8-hex-char monotonic
/// counter suffix = 20 chars total. Sorts by deletion time when
/// stringified, matches the directory leaf order on disk.
///
/// Not a real ULID (no Crockford base32, no 80-bit randomness), but
/// fits the requirements SPEC §4.7.2 names without pulling a new
/// crate. If we ever need crypto-quality randomness, swap to the
/// `ulid` crate — every call site goes through `generate()` so the
/// change is a one-file diff.
pub fn generate() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    // Process-local counter prevents collisions within the same
    // millisecond from the same process. Cross-process collisions in
    // the same millisecond are tolerated — the manifest writer's lock
    // serializes appends, and even an identical id with different
    // grave_path is recoverable (operator inspects `meta.json`).
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{millis:012X}{counter:08X}")
}

static COUNTER: AtomicU16 = AtomicU16::new(0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_within_a_process() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            assert!(ids.insert(generate()), "duplicate id within process");
        }
    }

    #[test]
    fn ids_are_lex_sortable_by_time() {
        let first = generate();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = generate();
        assert!(
            second > first,
            "later id ({second}) should sort after earlier ({first})"
        );
    }
}
