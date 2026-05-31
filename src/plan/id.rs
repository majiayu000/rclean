use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn generate_candidate_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let timestamp_ms = now.as_millis() & ((1u128 << 48) - 1);
    let counter = PLAN_ID_COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    let entropy = ((std::process::id() as u128 & 0xffff) << 64)
        | ((counter & 0x0000_ffff_ffff_ffff) << 16)
        | (now.subsec_nanos() as u128 & 0xffff);
    encode_ulid((timestamp_ms << 80) | entropy)
}

fn encode_ulid(mut value: u128) -> String {
    const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut out = [b'0'; 26];
    for index in (0..out.len()).rev() {
        out[index] = CROCKFORD[(value & 0b1_1111) as usize];
        value >>= 5;
    }
    String::from_utf8(out.to_vec()).expect("ULID alphabet is valid UTF-8")
}

static PLAN_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
