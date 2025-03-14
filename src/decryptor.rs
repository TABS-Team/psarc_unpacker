use aes::Aes256;
use cfb_mode::Decryptor;
use aes::cipher::{KeyIvInit, AsyncStreamCipher, generic_array::GenericArray};
use std::io::{self, Cursor, Read, Seek};

/// Constants for PSARC decryption.
pub const PSARC_KEY: [u8; 32] = [
    0xC5, 0x3D, 0xB2, 0x38, 0x70, 0xA1, 0xA2, 0xF7,
    0x1C, 0xAE, 0x64, 0x06, 0x1F, 0xDD, 0x0E, 0x11,
    0x57, 0x30, 0x9D, 0xC8, 0x52, 0x04, 0xD4, 0xC5,
    0xBF, 0xDF, 0x25, 0x09, 0x0D, 0xF2, 0x57, 0x2C,
];

/// For PSARC decryption, the IV is all zero.
pub const PSARC_IV: [u8; 16] = [0; 16];

/// A DecryptStream in PSARC mode.
/// It decrypts a fixed-length block of data from an input stream and provides a
/// Cursor over the decrypted data.
pub struct DecryptStream {
    pub reader: Cursor<Vec<u8>>,
}

impl DecryptStream {
    /// Creates a new DecryptStream for PSARC mode.
    ///
    /// * `input` - the input stream (which must be positioned at the beginning of the encrypted data)
    /// * `length` - the number of bytes of encrypted data to read
    ///
    /// This function reads the encrypted data into memory, decrypts it using AES-256 CFB with a zero IV,
    /// and returns a DecryptStream that provides access to the decrypted data.
    pub fn new_psarc<R: Read + Seek>(mut input: R, length: usize) -> io::Result<Self> {
        // Read the encrypted data into a buffer.
        let mut encrypted_data = vec![0u8; length];
        input.read_exact(&mut encrypted_data)?;
        
        // Create a GenericArray from the key and IV.
        let key = GenericArray::from_slice(&PSARC_KEY);
        let iv = GenericArray::from_slice(&PSARC_IV);
        
        // Create a new decryptor using AES-256 in CFB mode.
        let cipher = Decryptor::<Aes256>::new(key, iv);
        
        // Decrypt in place.
        cipher.decrypt(&mut encrypted_data);
        
        // Wrap the decrypted data in a Cursor.
        let reader = Cursor::new(encrypted_data);
        Ok(DecryptStream { reader })
    }
}
