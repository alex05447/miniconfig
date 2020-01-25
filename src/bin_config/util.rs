pub(crate) fn string_hash_fnv1a(string: &str) -> u32 {
    const FNV1A_PRIME: u32 = 0x0100_0193;
    const FNV1A_SEED: u32 = 0x811C_9DC5;

    let mut hash = FNV1A_SEED;

    for byte in string.as_bytes() {
        hash = (hash ^ *byte as u32).wrapping_mul(FNV1A_PRIME);
    }

    hash
}
