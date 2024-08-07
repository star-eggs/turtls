//! The ChaCha20 stream cipher.
const fn quarter_round(mut a: u32, mut b: u32, mut c: u32, mut d: u32) -> (u32, u32, u32, u32) {
    a = a.wrapping_add(b);
    d ^= a;
    d = d.rotate_left(16);
    c = c.wrapping_add(d);
    b ^= c;
    b = b.rotate_left(12);
    a = a.wrapping_add(b);
    d ^= a;
    d = d.rotate_left(8);
    c = c.wrapping_add(d);
    b ^= c;
    b = b.rotate_left(7);
    (a, b, c, d)
}

fn inner_block(state: &mut [u32; 16]) {
    (state[0], state[4], state[8], state[12]) =
        quarter_round(state[0], state[4], state[8], state[12]);

    (state[1], state[5], state[9], state[13]) =
        quarter_round(state[1], state[5], state[9], state[13]);

    (state[2], state[6], state[10], state[14]) =
        quarter_round(state[2], state[6], state[10], state[14]);

    (state[3], state[7], state[11], state[15]) =
        quarter_round(state[3], state[7], state[11], state[15]);

    (state[0], state[5], state[10], state[15]) =
        quarter_round(state[0], state[5], state[10], state[15]);

    (state[1], state[6], state[11], state[12]) =
        quarter_round(state[1], state[6], state[11], state[12]);

    (state[2], state[7], state[8], state[13]) =
        quarter_round(state[2], state[7], state[8], state[13]);

    (state[3], state[4], state[9], state[14]) =
        quarter_round(state[3], state[4], state[9], state[14]);
}

fn block(key: [u8; 32], nonce: [u8; 12], counter: u32) -> [u8; 64] {
    let mut state = config_state(key, nonce, counter);

    let mut working_state = state;

    for _ in 0..10 {
        inner_block(&mut working_state);
    }

    // add original state and working state
    for (state_byte, working_state_byte) in state.iter_mut().zip(working_state) {
        *state_byte = state_byte.wrapping_add(working_state_byte);
    }
    let mut output = [0u8; 64];
    for (output_chunk, state_chunk) in
        // TODO: use `array_chunks` once stabilized
        output.chunks_exact_mut(4).zip(state.iter())
    {
        output_chunk.copy_from_slice(&state_chunk.to_le_bytes());
    }
    output
}

fn config_state(key: [u8; 32], nonce: [u8; 12], counter: u32) -> [u32; 16] {
    // TODO: consider using uninitialized array
    let mut state = [
        0x61707865, 0x3320646e, 0x79622d32, 0x6b206574, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    for (key_chunk, state_chunk) in
        // TODO: use `array_chunks` once stabilized
        key.chunks_exact(4).zip(state[4..12].iter_mut())
    {
        // we can safely unwrap because `key_chunk` is guaranteed to have a length of 4
        *state_chunk = u32::from_le_bytes(key_chunk.try_into().unwrap());
    }
    state[12] = counter;
    for (nonce_chunk, state_chunk) in
        // TODO: use `array_chunks` once stabilized
        nonce.chunks_exact(4).zip(state[13..].iter_mut())
    {
        // we can safely unwrap because `key_chunk` is guaranteed to have a length of 4
        *state_chunk = u32::from_le_bytes(nonce_chunk.try_into().unwrap());
    }
    state
}

/// Encrypts `msg` inline
///
/// `counter` can be any number, often `0` or `1`
///
/// WARNING: users MUST NOT use the same `nonce`
/// more than once with the same key
pub fn encrypt_inline(msg: &mut [u8], key: [u8; 32], nonce: [u8; 12], counter: u32) {
    for (index, chunk) in msg.chunks_mut(64).enumerate() {
        let key_stream = block(key, nonce, counter + index as u32);
        for (chunk_byte, key_stream_byte) in chunk.iter_mut().zip(key_stream.iter()) {
            *chunk_byte ^= key_stream_byte
        }
    }
}

/// Encrypts `msg`, writing the encrypted msg to `buf`
///
/// # Panics
///
/// The function will panic if `msg.len()` > `buf.len()`
///
/// # Usage notes
///
/// `counter` can be any number, often `0` or `1`
///
/// WARNING: users MUST NOT use the same `nonce`
/// more than once with the same key
pub fn encrypt(msg: &[u8], key: [u8; 32], nonce: [u8; 12], counter: u32, buf: &mut [u8]) {
    buf[..msg.len()].copy_from_slice(msg);
    encrypt_inline(buf, key, nonce, counter);
}

#[cfg(test)]
mod tests {
    #[test]
    fn quarter_round() {
        let a = 0x11111111;
        let b = 0x01020304;
        let c = 0x9b8d6f43;
        let d = 0x01234567;

        let a2 = 0xea2a92f4;
        let b2 = 0xcb1cf8ce;
        let c2 = 0x4581472e;
        let d2 = 0x5881c4bb;

        assert_eq!((a2, b2, c2, d2), super::quarter_round(a, b, c, d));
    }

    #[test]
    fn block() {
        let key = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let counter = 1;
        let nonce = [
            0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00,
        ];
        let output_state = [
            0x10, 0xf1, 0xe7, 0xe4, 0xd1, 0x3b, 0x59, 0x15, 0x50, 0x0f, 0xdd, 0x1f, 0xa3, 0x20,
            0x71, 0xc4, 0xc7, 0xd1, 0xf4, 0xc7, 0x33, 0xc0, 0x68, 0x03, 0x04, 0x22, 0xaa, 0x9a,
            0xc3, 0xd4, 0x6c, 0x4e, 0xd2, 0x82, 0x64, 0x46, 0x07, 0x9f, 0xaa, 0x09, 0x14, 0xc2,
            0xd7, 0x05, 0xd9, 0x8b, 0x02, 0xa2, 0xb5, 0x12, 0x9c, 0xd1, 0xde, 0x16, 0x4e, 0xb9,
            0xcb, 0xd0, 0x83, 0xe8, 0xa2, 0x50, 0x3c, 0x4e,
        ];
        assert_eq!(output_state, super::block(key, nonce, counter));
    }

    #[test]
    fn chacha20() {
        let key = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let counter = 1;
        let nonce = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut plain_text = [
            0x4c, 0x61, 0x64, 0x69, 0x65, 0x73, 0x20, 0x61, 0x6e, 0x64, 0x20, 0x47, 0x65, 0x6e,
            0x74, 0x6c, 0x65, 0x6d, 0x65, 0x6e, 0x20, 0x6f, 0x66, 0x20, 0x74, 0x68, 0x65, 0x20,
            0x63, 0x6c, 0x61, 0x73, 0x73, 0x20, 0x6f, 0x66, 0x20, 0x27, 0x39, 0x39, 0x3a, 0x20,
            0x49, 0x66, 0x20, 0x49, 0x20, 0x63, 0x6f, 0x75, 0x6c, 0x64, 0x20, 0x6f, 0x66, 0x66,
            0x65, 0x72, 0x20, 0x79, 0x6f, 0x75, 0x20, 0x6f, 0x6e, 0x6c, 0x79, 0x20, 0x6f, 0x6e,
            0x65, 0x20, 0x74, 0x69, 0x70, 0x20, 0x66, 0x6f, 0x72, 0x20, 0x74, 0x68, 0x65, 0x20,
            0x66, 0x75, 0x74, 0x75, 0x72, 0x65, 0x2c, 0x20, 0x73, 0x75, 0x6e, 0x73, 0x63, 0x72,
            0x65, 0x65, 0x6e, 0x20, 0x77, 0x6f, 0x75, 0x6c, 0x64, 0x20, 0x62, 0x65, 0x20, 0x69,
            0x74, 0x2e,
        ];
        let cipher_text = [
            0x6e, 0x2e, 0x35, 0x9a, 0x25, 0x68, 0xf9, 0x80, 0x41, 0xba, 0x07, 0x28, 0xdd, 0x0d,
            0x69, 0x81, 0xe9, 0x7e, 0x7a, 0xec, 0x1d, 0x43, 0x60, 0xc2, 0x0a, 0x27, 0xaf, 0xcc,
            0xfd, 0x9f, 0xae, 0x0b, 0xf9, 0x1b, 0x65, 0xc5, 0x52, 0x47, 0x33, 0xab, 0x8f, 0x59,
            0x3d, 0xab, 0xcd, 0x62, 0xb3, 0x57, 0x16, 0x39, 0xd6, 0x24, 0xe6, 0x51, 0x52, 0xab,
            0x8f, 0x53, 0x0c, 0x35, 0x9f, 0x08, 0x61, 0xd8, 0x07, 0xca, 0x0d, 0xbf, 0x50, 0x0d,
            0x6a, 0x61, 0x56, 0xa3, 0x8e, 0x08, 0x8a, 0x22, 0xb6, 0x5e, 0x52, 0xbc, 0x51, 0x4d,
            0x16, 0xcc, 0xf8, 0x06, 0x81, 0x8c, 0xe9, 0x1a, 0xb7, 0x79, 0x37, 0x36, 0x5a, 0xf9,
            0x0b, 0xbf, 0x74, 0xa3, 0x5b, 0xe6, 0xb4, 0x0b, 0x8e, 0xed, 0xf2, 0x78, 0x5e, 0x42,
            0x87, 0x4d,
        ];
        super::encrypt_inline(&mut plain_text, key, nonce, counter);
        assert_eq!(plain_text, cipher_text);
    }
}
