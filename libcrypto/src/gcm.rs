//! Gallois/counter mode for AES
//!
//! This module implements Gallois/counter mode for AES.
//! It provides efficient encryption/decryption and authentication
//! for data of arbitrary length.
//!
//! # Examples
//!
//! ```
//! use libcrypto::gcm::Gcm;
//! use libcrypto::aes::Aes128;
//!
//! let plain_text = "Top secret message".as_bytes();
//! let additional_data = "Public information".as_bytes();
//!
//! let key = [
//!     0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c, 0x6d, 0x6a, 0x8f,
//!     0x94, 0x67, 0x30, 0x83, 0x08,
//! ];
//!
//! let init_vector = [
//!     0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad, 0xde, 0xca, 0xf8,
//!     0x88,
//! ];
//!
//! let cipher = Gcm::<Aes128>::new(key);
//!
//! let (encrypted_message, generated_tag) =
//!     cipher.encrypt(&plain_text, &additional_data, &init_vector);
//!
//! let tag = [
//!     0x4, 0xca, 0x8f, 0xa2, 0x89, 0xa6, 0xac, 0x83, 0x84, 0x68, 0x71,
//!     0x46, 0x2a, 0xda, 0x12, 0x67,
//! ];
//!
//! let cipher_text = [
//!     0xcf, 0xdd, 0x5c, 0xc7, 0xaa, 0x96, 0x11, 0xb3, 0x8b, 0x5f, 0x8,
//!     0x1f, 0x4e, 0x56, 0x81, 0x67, 0x2, 0x68,
//! ];
//!
//! assert_eq!(encrypted_message, cipher_text);
//! assert_eq!(generated_tag, tag);
//!
//! let unencrypted_message = cipher.decrypt(
//!     &encrypted_message, additional_data, &init_vector, &generated_tag
//!     ).expect("Our message has been modified!");
//!
//! assert_eq!(plain_text, "Top secret message".as_bytes());
//! ```
use crate::aes::{self, Aes128, Aes192, Aes256, AesCipher};

const R: u128 = 0xe1 << 120;
/// The size of an initialization vector, in bytes
pub const IV_SIZE: usize = 12;

/// An error that is returned when an encrypted message's tag
/// does not match its generated tag
///
/// If this error is found, the message cannot be considered safe
#[derive(Debug)]
pub struct BadData;

impl std::fmt::Display for BadData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tag did not match data")
    }
}

impl std::error::Error for BadData {}

/// A type that allows for authenticated
/// encryption and decryption in GCM via AES.
///
/// See `Gcm`'s implementations for examples.
pub struct Gcm<C: aes::AesCipher> {
    cipher: C,
    h: u128,
}

impl Gcm<aes::Aes128> {
    /// create a new `Gcm` using AES-128 as the cipher
    pub fn new(key: [u8; Aes128::KEY_SIZE]) -> Gcm<aes::Aes128> {
        let cipher = Aes128::new(key);
        let mut h = [0u8; aes::BLOCK_SIZE];
        cipher.encrypt_inline(&mut h);

        Gcm {
            cipher,
            h: u128::from_be_bytes(h),
        }
    }
}

impl Gcm<aes::Aes192> {
    /// create a new `Gcm` using AES-192 as the cipher
    pub fn new(key: [u8; Aes192::KEY_SIZE]) -> Gcm<aes::Aes192> {
        let cipher = aes::Aes192::new(key);
        let mut h = [0u8; aes::BLOCK_SIZE];
        cipher.encrypt_inline(&mut h);

        Gcm {
            cipher,
            h: u128::from_be_bytes(h),
        }
    }
}

impl Gcm<aes::Aes256> {
    /// create a new `Gcm` using AES-256 as the cipher
    pub fn new(key: [u8; Aes256::KEY_SIZE]) -> Gcm<aes::Aes256> {
        let cipher = Aes256::new(key);
        let mut h = [0u8; aes::BLOCK_SIZE];
        cipher.encrypt_inline(&mut h);

        Gcm {
            cipher,
            h: u128::from_be_bytes(h),
        }
    }
}

impl<C: aes::AesCipher> Gcm<C> {
    /// Encrypts `plain_text` inline, and generates an authentication tag
    /// for `plain_text` and `add_data`.
    ///
    /// WARNING: for security purposes,
    /// users MUST NOT use the same `init_vector` twice for the same key.
    pub fn encrypt_inline(
        &self,
        plain_text: &mut [u8],
        add_data: &[u8],
        init_vector: &[u8; IV_SIZE],
    ) -> [u8; aes::BLOCK_SIZE] {
        let counter = {
            let mut counter = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
            counter[..init_vector.len()].copy_from_slice(init_vector);
            counter
        };
        self.xor_bit_stream(plain_text, &counter);

        self.g_hash(plain_text, add_data, &counter)
    }

    /// Encrypts data, returning encrypted data and an authentication tag
    ///
    /// WARNING: for security purposes,
    /// users MUST NOT use the same `init_vector` twice for the same key.
    pub fn encrypt(
        &self,
        plain_text: &[u8],
        add_data: &[u8],
        init_vector: &[u8; IV_SIZE],
    ) -> (Vec<u8>, [u8; aes::BLOCK_SIZE]) {
        let mut buffer = plain_text.to_vec();
        let tag = self.encrypt_inline(&mut buffer, add_data, init_vector);
        (buffer, tag)
    }

    /// Decrypts `cipher_text` inline.
    ///
    pub fn decrypt_inline(
        &self,
        cipher_text: &mut [u8],
        add_data: &[u8],
        init_vector: &[u8; IV_SIZE],
        tag: &[u8; aes::BLOCK_SIZE],
    ) -> Result<(), BadData> {
        let counter = {
            let mut counter = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
            counter[..init_vector.len()].copy_from_slice(init_vector);
            counter
        };
        if self.g_hash(cipher_text, add_data, &counter) != *tag {
            return Err(BadData);
        }
        self.xor_bit_stream(cipher_text, &counter);
        Ok(())
    }

    /// Retuns an `Err(BadData)` if the message has been modified
    ///
    /// # Examples
    ///
    /// We've just recieved a message. Let's decrypt it!
    ///
    /// Here's our message:
    /// ```
    /// // our message:
    /// let cipher_text = [
    ///     0xcf, 0xdd, 0x5c, 0xc7, 0xaa, 0x96, 0x11, 0xb3, 0x8b, 0x5f, 0x8,
    ///     0x1f, 0x4e, 0x56, 0x81, 0x67, 0x2, 0x68,
    /// ];
    /// let tag = [
    ///     0x4, 0xca, 0x8f, 0xa2, 0x89, 0xa6, 0xac, 0x83, 0x84, 0x68, 0x71,
    ///     0x46, 0x2a, 0xda, 0x12, 0x67,
    /// ];
    /// let add_data = "Public information".as_bytes();
    /// let init_vector = [
    ///     0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad, 0xde, 0xca, 0xf8,
    ///     0x88,
    /// ];
    /// ```
    /// Decrypt our message:
    /// ```
    /// use libcrypto::aes::Aes128;
    /// use libcrypto::gcm::Gcm;
    ///
    /// // our key has to be the same key used to encrypt the message
    /// let key = [
    ///     0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c, 0x6d, 0x6a, 0x8f,
    ///     0x94, 0x67, 0x30, 0x83, 0x08,
    /// ];
    ///
    /// let init_vector = [
    ///     0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad, 0xde, 0xca, 0xf8,
    ///     0x88,
    /// ];
    ///
    /// let cipher = Gcm::<Aes128>::new(key);
    ///
    /// # // our message:
    /// # let cipher_text = [
    /// #    0xcf, 0xdd, 0x5c, 0xc7, 0xaa, 0x96, 0x11, 0xb3, 0x8b, 0x5f, 0x8,
    /// #    0x1f, 0x4e, 0x56, 0x81, 0x67, 0x2, 0x68,
    /// # ];
    /// # let tag = [
    /// #     0x4, 0xca, 0x8f, 0xa2, 0x89, 0xa6, 0xac, 0x83, 0x84, 0x68, 0x71,
    /// #     0x46, 0x2a, 0xda, 0x12, 0x67,
    /// # ];
    /// # let add_data = "Public information".as_bytes();
    /// #
    /// // decrypt the message
    /// let plain_text = match cipher.decrypt(
    ///     &cipher_text, &add_data, &init_vector, &tag
    ///     ) {
    ///     Ok(plain_text) => plain_text, // success! Our data is intact
    ///     Err(e) => panic!("{}", e),
    ///     };
    /// ```
    ///
    /// If we modifiy the message before decryption, we get an error:
    ///
    /// ```should_panic
    /// # use libcrypto::aes::Aes128;
    /// # use libcrypto::gcm::Gcm;
    /// #
    /// # let key = [
    /// #   0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15,
    /// #   0x88, 0x09, 0xcf, 0x4f, 0x3c,
    /// # ];
    /// #
    /// # let cipher = Gcm::<Aes128>::new(key);
    /// #
    /// # // our message:
    /// # let mut cipher_text = [
    /// #     0xcf, 0xdd, 0x5c, 0xc7, 0xaa, 0x96, 0x11, 0xb3, 0x8b, 0x5f, 0x8,
    /// #     0x1f, 0x4e, 0x56, 0x81, 0x67, 0x2, 0x68,
    /// # ];
    /// # let tag = [
    /// #     0x4, 0xca, 0x8f, 0xa2, 0x89, 0xa6, 0xac, 0x83, 0x84, 0x68, 0x71,
    /// #     0x46, 0x2a, 0xda, 0x12, 0x67,
    /// # ];
    /// # let add_data = "Public information".as_bytes();
    /// # let init_vector = [
    /// #     0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad, 0xde, 0xca, 0xf8,
    /// #     0x88,
    /// # ];
    /// #
    /// // flip a bit
    /// cipher_text[10] ^= 1 << 4;
    ///
    /// // decrypt the message
    /// let plain_text = match cipher.decrypt(
    ///     &cipher_text, &add_data, &init_vector, &tag
    ///     ) {
    ///     Ok(plain_text) => plain_text,
    ///     Err(e) => panic!("{}", e), // uh oh, our data has been modified!
    ///     };
    /// ```
    ///
    pub fn decrypt(
        &self,
        cipher_text: &[u8],
        add_data: &[u8],
        init_vector: &[u8; IV_SIZE],
        tag: &[u8; aes::BLOCK_SIZE],
    ) -> Result<Vec<u8>, BadData> {
        let mut buffer = cipher_text.to_vec();
        self.decrypt_inline(&mut buffer, add_data, init_vector, tag)?;
        Ok(buffer)
    }

    /// Encrypts or decrypts `data` in counter mode.
    ///
    /// Because XOR is its own inverse,
    /// the same operation can be used for encryption and decryption
    ///
    /// This is a linear operation.
    ///
    /// This process can be parallel-ized,
    /// but that has not been implemented yet.
    fn xor_bit_stream(&self, data: &mut [u8], counter: &[u8; aes::BLOCK_SIZE]) {
        let iv_as_int = u128::from_be_bytes(*counter);

        for (counter, block) in data.chunks_mut(aes::BLOCK_SIZE).enumerate() {
            let mut stream = (iv_as_int + 1 + counter as u128).to_be_bytes();
            self.cipher.encrypt_inline(&mut stream);

            for (data_byte, stream_byte) in block.iter_mut().zip(stream) {
                *data_byte ^= stream_byte;
            }
        }
    }

    /// produce an authentication tag for given data
    /// this tag can be used to verify the authenticity of the data
    fn g_hash(
        &self,
        cipher_text: &[u8],
        add_data: &[u8],
        counter: &[u8; aes::BLOCK_SIZE],
    ) -> [u8; aes::BLOCK_SIZE] {
        let mut tag = 0u128;

        for block in add_data.chunks_exact(aes::BLOCK_SIZE) {
            add_block(&mut tag, block.try_into().unwrap(), self.h);
        }

        let last_block = {
            let end = add_data.len() % aes::BLOCK_SIZE;
            let mut last_block = [0u8; aes::BLOCK_SIZE];
            last_block[..end]
                .copy_from_slice(&add_data[add_data.len() - end..]);
            last_block
        };

        add_block(&mut tag, last_block, self.h);

        for block in cipher_text.chunks_exact(aes::BLOCK_SIZE) {
            add_block(&mut tag, block.try_into().unwrap(), self.h);
        }

        let last_block = {
            let end = cipher_text.len() % aes::BLOCK_SIZE;
            let mut last_block = [0u8; aes::BLOCK_SIZE];
            last_block[..end]
                .copy_from_slice(&cipher_text[cipher_text.len() - end..]);
            last_block
        };

        add_block(&mut tag, last_block, self.h);

        tag ^= ((add_data.len() as u128 * 8) << 64)
            + cipher_text.len() as u128 * 8;
        tag = gf_2to128_mult(tag, self.h);

        let encrypted_iv = u128::from_be_bytes(self.cipher.encrypt(counter));

        tag ^= encrypted_iv;
        tag.to_be_bytes()
    }
}

/// Multiplication in GF(2^128)
///
/// Cannot overflow
fn gf_2to128_mult(a: u128, b: u128) -> u128 {
    let mut product = 0;
    let mut temp = a;
    for i in (0..128).rev() {
        if b & (1 << i) == 1 << i {
            product ^= temp;
        }
        if temp & 1 == 0 {
            temp >>= 1;
        } else {
            temp = (temp >> 1) ^ R;
        }
    }
    product
}

/// A helper function for g_hash()
fn add_block(tag: &mut u128, block: [u8; aes::BLOCK_SIZE], h: u128) {
    *tag ^= u128::from_be_bytes(block);
    *tag = gf_2to128_mult(*tag, h);
}

#[cfg(test)]
mod tests {
    use super::Gcm;
    use crate::aes::Aes128;

    #[test]
    fn ctr_mode() {
        let key = [
            0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c, 0x6d, 0x6a, 0x8f,
            0x94, 0x67, 0x30, 0x83, 0x08,
        ];
        let mut plain_text = [
            0xd9, 0x31, 0x32, 0x25, 0xf8, 0x84, 0x06, 0xe5, 0xa5, 0x59, 0x09,
            0xc5, 0xaf, 0xf5, 0x26, 0x9a, 0x86, 0xa7, 0xa9, 0x53, 0x15, 0x34,
            0xf7, 0xda, 0x2e, 0x4c, 0x30, 0x3d, 0x8a, 0x31, 0x8a, 0x72, 0x1c,
            0x3c, 0x0c, 0x95, 0x95, 0x68, 0x09, 0x53, 0x2f, 0xcf, 0x0e, 0x24,
            0x49, 0xa6, 0xb5, 0x25, 0xb1, 0x6a, 0xed, 0xf5, 0xaa, 0x0d, 0xe6,
            0x57, 0xba, 0x63, 0x7b, 0x39,
        ];
        let counter = [
            0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad, 0xde, 0xca, 0xf8,
            0x88, 0x00, 0x00, 0x00, 0x01,
        ];
        let cipher_text = [
            0x42, 0x83, 0x1e, 0xc2, 0x21, 0x77, 0x74, 0x24, 0x4b, 0x72, 0x21,
            0xb7, 0x84, 0xd0, 0xd4, 0x9c, 0xe3, 0xaa, 0x21, 0x2f, 0x2c, 0x02,
            0xa4, 0xe0, 0x35, 0xc1, 0x7e, 0x23, 0x29, 0xac, 0xa1, 0x2e, 0x21,
            0xd5, 0x14, 0xb2, 0x54, 0x66, 0x93, 0x1c, 0x7d, 0x8f, 0x6a, 0x5a,
            0xac, 0x84, 0xaa, 0x05, 0x1b, 0xa3, 0x0b, 0x39, 0x6a, 0x0a, 0xac,
            0x97, 0x3d, 0x58, 0xe0, 0x91,
        ];
        let cipher = Gcm::<Aes128>::new(key);
        cipher.xor_bit_stream(&mut plain_text, &counter);
        assert_eq!(plain_text, cipher_text);
    }

    #[test]
    fn g_hash() {
        let key = [
            0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c, 0x6d, 0x6a, 0x8f,
            0x94, 0x67, 0x30, 0x83, 0x08,
        ];
        let cipher = Gcm::<Aes128>::new(key);

        let counter = [
            0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad, 0xde, 0xca, 0xf8,
            0x88, 0x00, 0x00, 0x00, 0x01,
        ];

        let cipher_text = [
            0x42, 0x83, 0x1e, 0xc2, 0x21, 0x77, 0x74, 0x24, 0x4b, 0x72, 0x21,
            0xb7, 0x84, 0xd0, 0xd4, 0x9c, 0xe3, 0xaa, 0x21, 0x2f, 0x2c, 0x02,
            0xa4, 0xe0, 0x35, 0xc1, 0x7e, 0x23, 0x29, 0xac, 0xa1, 0x2e, 0x21,
            0xd5, 0x14, 0xb2, 0x54, 0x66, 0x93, 0x1c, 0x7d, 0x8f, 0x6a, 0x5a,
            0xac, 0x84, 0xaa, 0x05, 0x1b, 0xa3, 0x0b, 0x39, 0x6a, 0x0a, 0xac,
            0x97, 0x3d, 0x58, 0xe0, 0x91,
        ];
        let tag = [
            0x5b, 0xc9, 0x4f, 0xbc, 0x32, 0x21, 0xa5, 0xdb, 0x94, 0xfa, 0xe9,
            0x5a, 0xe7, 0x12, 0x1a, 0x47,
        ];

        let add_data = [
            0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef, 0xfe, 0xed, 0xfa,
            0xce, 0xde, 0xad, 0xbe, 0xef, 0xab, 0xad, 0xda, 0xd2,
        ];

        let h = 0xb83b533708bf535d0aa6e52980d53b78;
        assert_eq!(cipher.h, h);

        assert_eq!(tag, cipher.g_hash(&cipher_text, &add_data, &counter));
    }

    #[test]
    fn mult() {
        let a = 0x66e94bd4ef8a2c3b884cfa59ca342b2e;
        let b = 0x0388dace60b6a392f328c2b971b2fe78;
        let product = 0x5e2ec746917062882c85b0685353deb7;
        assert_eq!(super::gf_2to128_mult(a, b), product);
    }

    #[test]
    fn encrypt() {
        let key = [
            0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c, 0x6d, 0x6a, 0x8f,
            0x94, 0x67, 0x30, 0x83, 0x08,
        ];
        let cipher = Gcm::<Aes128>::new(key);

        let init_vector = [
            0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad, 0xde, 0xca, 0xf8,
            0x88,
        ];

        let mut plain_text = [
            0xd9, 0x31, 0x32, 0x25, 0xf8, 0x84, 0x06, 0xe5, 0xa5, 0x59, 0x09,
            0xc5, 0xaf, 0xf5, 0x26, 0x9a, 0x86, 0xa7, 0xa9, 0x53, 0x15, 0x34,
            0xf7, 0xda, 0x2e, 0x4c, 0x30, 0x3d, 0x8a, 0x31, 0x8a, 0x72, 0x1c,
            0x3c, 0x0c, 0x95, 0x95, 0x68, 0x09, 0x53, 0x2f, 0xcf, 0x0e, 0x24,
            0x49, 0xa6, 0xb5, 0x25, 0xb1, 0x6a, 0xed, 0xf5, 0xaa, 0x0d, 0xe6,
            0x57, 0xba, 0x63, 0x7b, 0x39,
        ];
        let tag = [
            0x5b, 0xc9, 0x4f, 0xbc, 0x32, 0x21, 0xa5, 0xdb, 0x94, 0xfa, 0xe9,
            0x5a, 0xe7, 0x12, 0x1a, 0x47,
        ];

        let add_data = [
            0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef, 0xfe, 0xed, 0xfa,
            0xce, 0xde, 0xad, 0xbe, 0xef, 0xab, 0xad, 0xda, 0xd2,
        ];
        let cipher_text = [
            0x42, 0x83, 0x1e, 0xc2, 0x21, 0x77, 0x74, 0x24, 0x4b, 0x72, 0x21,
            0xb7, 0x84, 0xd0, 0xd4, 0x9c, 0xe3, 0xaa, 0x21, 0x2f, 0x2c, 0x02,
            0xa4, 0xe0, 0x35, 0xc1, 0x7e, 0x23, 0x29, 0xac, 0xa1, 0x2e, 0x21,
            0xd5, 0x14, 0xb2, 0x54, 0x66, 0x93, 0x1c, 0x7d, 0x8f, 0x6a, 0x5a,
            0xac, 0x84, 0xaa, 0x05, 0x1b, 0xa3, 0x0b, 0x39, 0x6a, 0x0a, 0xac,
            0x97, 0x3d, 0x58, 0xe0, 0x91,
        ];
        assert_eq!(
            tag,
            cipher.encrypt_inline(&mut plain_text, &add_data, &init_vector)
        );
        assert_eq!(plain_text, cipher_text);
    }

    #[test]
    fn decrypt() {
        let key = [
            0xfe, 0xff, 0xe9, 0x92, 0x86, 0x65, 0x73, 0x1c, 0x6d, 0x6a, 0x8f,
            0x94, 0x67, 0x30, 0x83, 0x08,
        ];
        let cipher = Gcm::<Aes128>::new(key);

        let init_vector = [
            0xca, 0xfe, 0xba, 0xbe, 0xfa, 0xce, 0xdb, 0xad, 0xde, 0xca, 0xf8,
            0x88,
        ];

        let plain_text = [
            0xd9, 0x31, 0x32, 0x25, 0xf8, 0x84, 0x06, 0xe5, 0xa5, 0x59, 0x09,
            0xc5, 0xaf, 0xf5, 0x26, 0x9a, 0x86, 0xa7, 0xa9, 0x53, 0x15, 0x34,
            0xf7, 0xda, 0x2e, 0x4c, 0x30, 0x3d, 0x8a, 0x31, 0x8a, 0x72, 0x1c,
            0x3c, 0x0c, 0x95, 0x95, 0x68, 0x09, 0x53, 0x2f, 0xcf, 0x0e, 0x24,
            0x49, 0xa6, 0xb5, 0x25, 0xb1, 0x6a, 0xed, 0xf5, 0xaa, 0x0d, 0xe6,
            0x57, 0xba, 0x63, 0x7b, 0x39,
        ];
        let tag = [
            0x5b, 0xc9, 0x4f, 0xbc, 0x32, 0x21, 0xa5, 0xdb, 0x94, 0xfa, 0xe9,
            0x5a, 0xe7, 0x12, 0x1a, 0x47,
        ];

        let add_data = [
            0xfe, 0xed, 0xfa, 0xce, 0xde, 0xad, 0xbe, 0xef, 0xfe, 0xed, 0xfa,
            0xce, 0xde, 0xad, 0xbe, 0xef, 0xab, 0xad, 0xda, 0xd2,
        ];
        let mut cipher_text = [
            0x42, 0x83, 0x1e, 0xc2, 0x21, 0x77, 0x74, 0x24, 0x4b, 0x72, 0x21,
            0xb7, 0x84, 0xd0, 0xd4, 0x9c, 0xe3, 0xaa, 0x21, 0x2f, 0x2c, 0x02,
            0xa4, 0xe0, 0x35, 0xc1, 0x7e, 0x23, 0x29, 0xac, 0xa1, 0x2e, 0x21,
            0xd5, 0x14, 0xb2, 0x54, 0x66, 0x93, 0x1c, 0x7d, 0x8f, 0x6a, 0x5a,
            0xac, 0x84, 0xaa, 0x05, 0x1b, 0xa3, 0x0b, 0x39, 0x6a, 0x0a, 0xac,
            0x97, 0x3d, 0x58, 0xe0, 0x91,
        ];
        cipher
            .decrypt_inline(&mut cipher_text, &add_data, &init_vector, &tag)
            .unwrap();
        assert_eq!(plain_text, cipher_text);
    }
}
