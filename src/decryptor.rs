use aes::Aes256;
use cfb_mode::Decryptor;
use ctr::{Ctr128BE};
use ctr::cipher::{KeyIvInit, StreamCipher};
use flate2::read::{ZlibDecoder, DeflateDecoder};
use aes::cipher::{AsyncStreamCipher, generic_array::GenericArray};
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

/// Constant key for SNG decryption (SNG_KEY_PC)
pub const SNG_KEY_PC: [u8; 32] = [
    0xCB, 0x64, 0x8D, 0xF3, 0xD1, 0x2A, 0x16, 0xBF,
    0x71, 0x70, 0x14, 0x14, 0xE6, 0x96, 0x19, 0xEC,
    0x17, 0x1C, 0xCA, 0x5D, 0x2A, 0x14, 0x2E, 0x3E,
    0x59, 0xDE, 0x7A, 0xDD, 0xA1, 0x8A, 0x3A, 0x30,
];

/// A DecryptStream in PSARC or SNG mode.
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
        let mut encrypted_data = vec![0u8; length];
        input.read_exact(&mut encrypted_data)?;

        let key = GenericArray::from_slice(&PSARC_KEY);
        let iv = GenericArray::from_slice(&PSARC_IV);

        let cipher = Decryptor::<Aes256>::new(key, iv);

        cipher.decrypt(&mut encrypted_data);
        let reader = Cursor::new(encrypted_data);
        Ok(DecryptStream { reader })
    }

    /// Creates a new Rocksmith SNG decryption stream.
    ///
    /// # Arguments
    /// * `mut input` - The input stream (positioned at the beginning of the SNG file)
    /// * `length` - Total length in bytes (including the header)
    ///
    /// # Errors
    /// Returns an error if the header is invalid or I/O fails.
    pub fn new_sng<R: Read + Seek>(mut input: R, length: usize) -> io::Result<Self> {
        // --- Read Header (24 bytes) ---
        // 4 bytes: Identifier (must be 0x4A)
        // 4 bytes: Asset flags (bitfield; flag 0x1 indicates compression)
        // 16 bytes: Decryption IV
        let mut header = [0u8; 24];
        input.read_exact(&mut header)?;
        let mut header_cursor = Cursor::new(&header);

        // Identifier: read as u32 in little-endian
        let mut id_buf = [0u8; 4];
        header_cursor.read_exact(&mut id_buf)?;
        let identifier = u32::from_le_bytes(id_buf);
        if identifier != 0x4A {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not a valid sng file"));
        }

        // Asset flags (u32, little-endian)
        let mut flags_buf = [0u8; 4];
        header_cursor.read_exact(&mut flags_buf)?;
        let asset_flags = u32::from_le_bytes(flags_buf);

        // Decryption IV (16 bytes)
        let mut decrypt_iv = [0u8; 16];
        header_cursor.read_exact(&mut decrypt_iv)?;

        // --- Read Encrypted Data ---
        // Encrypted data length is total length minus header (24 bytes)
        let encrypted_length = length - 24;
        let mut encrypted_data = vec![0u8; encrypted_length];
        input.read_exact(&mut encrypted_data)?;

        // --- Decrypt using AES-256 in CTR mode ---
        // Use Ctr128BE (big-endian) to mimic the C# counter increment.
        type Aes256Ctr = Ctr128BE<Aes256>;
        let key = &SNG_KEY_PC;
        let mut cipher = Aes256Ctr::new(key.into(), (&decrypt_iv).into());
        cipher.apply_keystream(&mut encrypted_data);

        // --- Decompression if required ---
        // The C# code uses ZInputStream (which expects zlib-wrapped deflate).
        const SNG_ASSET_FLAG_COMPRESSED: u32 = 0x1;
        let final_data = if asset_flags & SNG_ASSET_FLAG_COMPRESSED != 0 {
            // The first 4 bytes of the decrypted data indicate the uncompressed size.
            if encrypted_data.len() < 4 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Decrypted data too short for uncompressed size",
                ));
            }
            let uncompressed_size = {
                let size_bytes = &encrypted_data[..4];
                u32::from_le_bytes(size_bytes.try_into().unwrap()) as usize
            };

            // The remainder (after the first 4 bytes) is compressed.
            let compressed_data = &encrypted_data[4..];
            let mut decoder = ZlibDecoder::new(compressed_data);
            let mut decompressed_data = Vec::with_capacity(uncompressed_size);
            decoder.read_to_end(&mut decompressed_data)?;
            decompressed_data
        } else {
            encrypted_data
        };

        let reader = Cursor::new(final_data);
        Ok(DecryptStream { reader })
    }
}
