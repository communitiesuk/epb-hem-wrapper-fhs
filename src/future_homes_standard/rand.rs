/// Module containing a function to generate a PCG64 seed from either an array of values, or a single value,
/// as would be accepted by the PCG64 implementation in NumPy
pub(crate) mod pcg_seed {
    pub fn numpy_like_seed_sequence(seed: u32) -> [u8; 32] {
        const INIT_A: u32 = 0x43b0d7ec;
        const MULT_A: u32 = 0x931e8875;
        const MULT_B: u32 = 0xca6b6141;
        const MULT_C: u32 = 0xed6516a5;

        let mut pool = [INIT_A; 4];

        let mut x = seed ^ INIT_A;
        x = x.wrapping_mul(MULT_A);
        x ^= x >> 15;

        pool[0] ^= x;

        let mut output_words = [0u32; 4];

        for i in 0..4 {
            let mut val = pool[i];
            val = val.wrapping_mul(MULT_B);
            val ^= val >> 13;
            val = val.wrapping_mul(MULT_C);
            val ^= val >> 15;

            output_words[i] = val;
        }

        let mut final_seed_bytes = [0u8; 32];
        for (i, word) in output_words.iter().enumerate() {
            let bytes = word.to_le_bytes();
            final_seed_bytes[i * 4..(i * 4) + 4].copy_from_slice(&bytes);
        }

        final_seed_bytes
    }

    pub fn numpy_like_seed_sequence_array(entropy: &[u32]) -> [u8; 32] {
        const INIT_A: u32 = 0x43b0d7e5;
        const MULT_A: u32 = 0x931e8875;
        const MULT_B: u32 = 0x58f38ded;
        const MIX_MULT_L: u32 = 0xca01f9dd;
        const MIX_MULT_R: u32 = 0x4973f715;

        let mut pool = [INIT_A; 4];

        for (i, &item) in entropy.iter().enumerate() {
            let mut x = item ^ INIT_A;
            x = x.wrapping_mul(MULT_A);
            x ^= x >> 16;

            let p_idx = i % 4;
            pool[p_idx] = pool[p_idx].wrapping_mul(MULT_A) ^ x;
        }

        for i in 0..4 {
            let left = pool[i].wrapping_mul(MIX_MULT_L);
            let right = pool[(i + 1) % 4].wrapping_mul(MIX_MULT_R);
            pool[i] = left ^ right;
            pool[i] ^= pool[i] >> 16;
        }

        let mut output_words = [0u32; 4];
        for i in 0..4 {
            let mut val = pool[i];
            val = val.wrapping_mul(MULT_B);
            val ^= val >> 16;
            val = val.wrapping_mul(MULT_B);
            val ^= val >> 16;
            output_words[i] = val;
        }

        let mut final_seed_bytes = [0u8; 32];
        for (i, word) in output_words.iter().enumerate() {
            final_seed_bytes[i * 4..(i * 4) + 4].copy_from_slice(&word.to_le_bytes());
        }

        final_seed_bytes
    }
}
