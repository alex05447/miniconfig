pub(crate) fn string_hash_fnv1a(string: &str) -> u32 {
    const FNV1A_PRIME: u32 = 0x0100_0193;
    const FNV1A_SEED: u32 = 0x811C_9DC5;

    let mut hash = FNV1A_SEED;

    for byte in string.as_bytes() {
        hash = (hash ^ *byte as u32).wrapping_mul(FNV1A_PRIME);
    }

    hash
}

pub(super) fn u32_to_bin(val: u32) -> u32 {
    u32::to_le(val)
    //u32::to_be(val)
}

pub(super) fn u32_to_bin_bytes(val: u32) -> [u8; 4] {
    u32::to_le_bytes(val)
    //u32::to_be_bytes(val)
}

pub(super) fn u32_from_bin(bin: u32) -> u32 {
    u32::from_le(bin)
    //u32::from_be(bin)
}

pub(super) fn u64_to_bin(val: u64) -> u64 {
    u64::to_le(val)
    //u64::to_be(val)
}

pub(super) fn u64_from_bin(bin: u64) -> u64 {
    u64::from_le(bin)
    //u64::from_be(bin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_hash_collisions() {
        // https://softwareengineering.stackexchange.com/questions/49550/which-hashing-algorithm-is-best-for-uniqueness-and-speed

        assert_eq!(string_hash_fnv1a("costarring"), string_hash_fnv1a("liquid"),);

        assert_eq!(
            string_hash_fnv1a("declinate"),
            string_hash_fnv1a("macallums"),
        );

        assert_eq!(string_hash_fnv1a("altarage"), string_hash_fnv1a("zinke"),);

        assert_eq!(string_hash_fnv1a("altarages"), string_hash_fnv1a("zinkes"),);

        assert_ne!(string_hash_fnv1a("foo"), string_hash_fnv1a("bar"),);
    }
}
