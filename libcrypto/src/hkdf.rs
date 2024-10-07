use crate::hash::{BlockHasher, BufHasher};
use crate::hmac::Hmac;

pub fn extract<const H_LEN: usize, const B_LEN: usize, H: BlockHasher<H_LEN, B_LEN>>(
    salt: &[u8],
    ikm: &[u8],
) -> [u8; H_LEN] {
    Hmac::<H_LEN, B_LEN, H>::auth(salt, ikm)
}

pub fn expand<
    const H_LEN: usize,
    const B_LEN: usize,
    const K_LEN: usize,
    H: BlockHasher<H_LEN, B_LEN>,
>(
    pr_key: &[u8; H_LEN],
    info: &[u8],
) -> [u8; K_LEN] {
    let mut key = [0; K_LEN];

    let mut prev_mac: &[u8] = &[];
    for (i, key_chunk) in key.chunks_mut(H_LEN).enumerate() {
        let mut hmac = Hmac::<H_LEN, B_LEN, BufHasher<H_LEN, B_LEN, H>>::new(pr_key);
        hmac.update_with(prev_mac);
        hmac.update_with(info);
        let mac = hmac.finish_with(&[i as u8 + 1]);
        key_chunk.copy_from_slice(&mac[..key_chunk.len()]);
        prev_mac = key_chunk;
    }
    key
}

#[cfg(test)]
pub mod tests {
    use crate::hash::Sha256;

    #[test]
    fn extract() {
        let salt = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
        ];
        let ikm = [
            0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
            0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b, 0x0b,
        ];
        let pseudo_random_key = [
            0x07, 0x77, 0x09, 0x36, 0x2c, 0x2e, 0x32, 0xdf, 0x0d, 0xdc, 0x3f, 0x0d, 0xc4, 0x7b,
            0xba, 0x63, 0x90, 0xb6, 0xc7, 0x3b, 0xb5, 0x0f, 0x9c, 0x31, 0x22, 0xec, 0x84, 0x4a,
            0xd7, 0xc2, 0xb3, 0xe5,
        ];
        assert_eq!(
            super::extract::<{ Sha256::HASH_SIZE }, { Sha256::BLOCK_SIZE }, Sha256>(&salt, &ikm),
            pseudo_random_key
        );
    }
}
