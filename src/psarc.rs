use std::io::{self, Read, Write, Seek, SeekFrom, Cursor};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::path::Path;
use flate2::read::DeflateDecoder;
use std::fs;
use tracing;
use serde_json;
use serde::Serialize;


use crate::decryptor::DecryptStream;
use crate::models::{
    Bpm, Phrase, Chord, ChordNotes, Vocal, SymbolsHeader, SymbolsTexture,
    SymbolDefinition, PhraseIteration, PhraseExtraInfoByLevel, NLinkedDifficulty,
    Action, Event, Tone, Dna, Section, Arrangement, Metadata, BinarySerializable,
    read_vec, 
};

bitflags::bitflags! {
    pub struct PsarcArchiveFlags: u32 {
        const NONE          = 0;
        const UNK1          = 1;
        const UNK2          = 2;
        const TOC_ENCRYPTED = 4;
        const UNK8          = 8;
        const UNK16         = 16;
        const UNK32         = 32;
        const UNK64         = 64;
        const UNK128        = 128;
    }
}

#[derive(Debug)]
pub struct PsarcFileHeader {
    pub identifier: String,
    pub version: u32,
    pub compression: String,
    pub toc_size: u32,
    pub toc_entry_size: u32,
    pub toc_offset: u64,
    pub entry_count: u32,
    pub block_size: u32,
    pub archive_flags: PsarcArchiveFlags,
}

impl PsarcFileHeader {
    /// Reads the header from a reader that implements `Read + Seek`.
    ///
    /// The header layout is 32 bytes:
    /// - 4 bytes: Identifier (ASCII)
    /// - 4 bytes: Version (big-endian u32)
    /// - 4 bytes: Compression (ASCII)
    /// - 4 bytes: TOCSize (big-endian u32)
    /// - 4 bytes: TOCEntrySize (big-endian u32)
    /// - 4 bytes: EntryCount (big-endian u32)
    /// - 4 bytes: BlockSize (big-endian u32)
    /// - 4 bytes: ArchiveFlags (big-endian u32)
    /// 
    /// After reading, the current file offset is stored as `toc_offset`.
    pub fn read_from<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        reader.seek(SeekFrom::Start(0))?;
        
        let mut identifier_buf = [0u8; 4];
        reader.read_exact(&mut identifier_buf)?;
        let identifier = String::from_utf8_lossy(&identifier_buf).to_string();
        
        let version = reader.read_u32::<BigEndian>()?;
        
        let mut compression_buf = [0u8; 4];
        reader.read_exact(&mut compression_buf)?;
        let compression = String::from_utf8_lossy(&compression_buf).to_string();
        
        let toc_size = reader.read_u32::<BigEndian>()?;
        let toc_entry_size = reader.read_u32::<BigEndian>()?;
        let entry_count = reader.read_u32::<BigEndian>()?;
        let block_size = reader.read_u32::<BigEndian>()?;
        
        let raw_archive_flags = reader.read_u32::<BigEndian>()?;
        let archive_flags = PsarcArchiveFlags::from_bits_truncate(raw_archive_flags);
        
        let toc_offset = reader.seek(SeekFrom::Current(0))?;
        
        Ok(PsarcFileHeader {
            identifier,
            version,
            compression,
            toc_size,
            toc_entry_size,
            toc_offset,
            entry_count,
            block_size,
            archive_flags,
        })
    }
}

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

#[derive(Debug, Clone)]
pub struct PsarcTOCEntry {
    pub index: i32,         // C# int → i32
    pub hash: String,       // 16-byte hash as an uppercase hex string
    pub start_block: u32,   // C# uint → u32
    pub length: u64,        // C# ulong → u64 (stored as 5 bytes, 40-bit value)
    pub offset: u64,        // C# ulong → u64 (stored as 5 bytes, 40-bit value)
    pub path: Option<String>,
}

/// Holds the TOC: the list of TOC entries, a flag for encryption, and the ZIP block sizes.
#[derive(Debug)]
pub struct PsarcTOC {
    pub entries: Vec<PsarcTOCEntry>,
    pub encrypted: bool,
    pub zip_block_sizes: Vec<u32>,
}

/// Helper: Reads a 40-bit unsigned integer (5 bytes) in BigEndian.
fn read_u40_be<R: Read>(reader: &mut R) -> io::Result<u64> {
    let mut buf = [0u8; 5];
    reader.read_exact(&mut buf)?;
    let value = ((buf[0] as u64) << 32)
        | ((buf[1] as u64) << 24)
        | ((buf[2] as u64) << 16)
        | ((buf[3] as u64) << 8)
        | (buf[4] as u64);
    Ok(value)
}

/// Helper: Reads a 24-bit unsigned integer (3 bytes) in BigEndian.
fn read_u24_be<R: Read>(reader: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 3];
    reader.read_exact(&mut buf)?;
    let value = ((buf[0] as u32) << 16)
        | ((buf[1] as u32) << 8)
        | (buf[2] as u32);
    Ok(value)
}

impl PsarcTOC {
    /// Reads the TOC from a reader (which must be positioned at the start of the TOC)
    /// using header information.
    ///
    /// If the header indicates that the TOC is encrypted, this function reads
    /// `header.toc_size` bytes from the input, decrypts them using your provided
    /// `DecryptStream::new_psarc`, and then wraps the decrypted data in a Cursor.
    pub fn read_from<R: Read + Seek>(mut reader: R, header: &PsarcFileHeader) -> io::Result<Self> {
        let encrypted = header.archive_flags.contains(PsarcArchiveFlags::TOC_ENCRYPTED);
        
        // If encrypted, use your decryptor to decrypt header.toc_size bytes.
        let mut toc_reader: Box<dyn ReadSeek> = if encrypted {
            let toc_size = header.toc_size as usize;
            let decrypt_stream = DecryptStream::new_psarc(&mut reader, toc_size)?;
            Box::new(decrypt_stream.reader)
        } else {
            Box::new(reader)
        };
        
        // Read entry count (4 bytes, BigEndian).
        let entry_count = header.entry_count;
        let mut entries = Vec::with_capacity(entry_count as usize);
        for i in 0..entry_count {
            let mut hash_bytes = [0u8; 16];
            toc_reader.read_exact(&mut hash_bytes)?;
            let hash = hash_bytes.iter()
                .map(|b| format!("{:02X}", b))
                .collect::<String>();
            let start_block = toc_reader.read_u32::<BigEndian>()?;
            let length = read_u40_be(&mut toc_reader)?;
            let offset = read_u40_be(&mut toc_reader)?;
            entries.push(PsarcTOCEntry {
                index: i as i32,
                hash,
                start_block,
                length,
                offset,
                path: None,
            });
        }
        
        // Compute the remaining bytes after the TOC entries.
        let toc_entries_bytes = (entry_count as usize) * (header.toc_entry_size as usize);
        let remaining = (header.toc_size as isize) - 32 - (toc_entries_bytes as isize);
        if remaining < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "TOC size too small"));
        }
        
        // Determine b_num = log256(header.block_size). For a block size of 65536, b_num should be 2.
        let b_num = (header.block_size as f64).log(256.0).round() as usize;
        let mut z_num = (remaining as usize) / b_num;
        // Clamp to a safe maximum (e.g. 500) to avoid huge allocations.
        z_num = z_num.min(500);
        let mut zip_block_sizes = Vec::with_capacity(z_num);
        for _ in 0..z_num {
            let size = match b_num {
                2 => toc_reader.read_u16::<BigEndian>()? as u32,
                3 => read_u24_be(&mut toc_reader)?,
                4 => toc_reader.read_u32::<BigEndian>()?,
                _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Unsupported block size base")),
            };
            zip_block_sizes.push(size);
        }
        
        Ok(PsarcTOC {
            entries,
            encrypted,
            zip_block_sizes,
        })
    }
}

pub trait PsarcAsset {
    fn read_from<R: Read + Seek>(&mut self, reader: &mut R, length: usize) -> io::Result<()>;
}

#[derive(Default, Debug)]
pub struct TextAsset {
    pub text: String,
    pub lines: Vec<String>,
}

#[derive(Default, Debug)]
pub struct SongArrangementAsset {
    pub text: String,
    pub lines: Vec<String>,
}

impl PsarcAsset for TextAsset {
    fn read_from<R: Read + Seek>(&mut self, reader: &mut R, length: usize) -> io::Result<()> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        self.text = String::from_utf8(buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.lines = self.text.lines().map(|s| s.to_string()).collect();
        Ok(())
    }
}

/// ----------------- SNG Asset -----------------
/// This struct represents the overall SNG asset. In the C# code the decryption/decompression
/// is done first and then the asset is read in order.
#[derive(Default, Debug, Serialize)]
pub struct SngAsset {
    pub bpms: Vec<Bpm>,
    pub phrases: Vec<Phrase>,
    pub chords: Vec<Chord>,
    pub chord_notes: Vec<ChordNotes>,
    pub vocals: Vec<Vocal>,
    pub symbol_headers: Option<Vec<SymbolsHeader>>,
    pub symbol_textures: Option<Vec<SymbolsTexture>>,
    pub symbol_definitions: Option<Vec<SymbolDefinition>>,
    pub phrase_iterations: Vec<PhraseIteration>,
    pub phrase_extra_info: Vec<PhraseExtraInfoByLevel>,
    pub nld: Vec<NLinkedDifficulty>,
    pub actions: Vec<Action>,
    pub events: Vec<Event>,
    pub tones: Vec<Tone>,
    pub dnas: Vec<Dna>,
    pub sections: Vec<Section>,
    pub arrangements: Vec<Arrangement>,
    pub metadata: Metadata,
}

/// For arrays that do not have a preceding count in the SNG file you might need to adjust
/// the reading functions accordingly. Here we assume that each “array” is preceded by an i32 count.
impl PsarcAsset for SngAsset {
    fn read_from<R: Read + Seek>(&mut self, reader: &mut R, length: usize) -> io::Result<()> {
        let mut decryptor = DecryptStream::new_sng(reader, length)?;
        self.bpms = read_vec(&mut decryptor.reader, Bpm::read_from)?;
        self.phrases = read_vec(&mut decryptor.reader, Phrase::read_from)?;
        self.chords = read_vec(&mut decryptor.reader, Chord::read_from)?;
        self.chord_notes = read_vec(&mut decryptor.reader, ChordNotes::read_from)?;
        self.vocals = read_vec(&mut decryptor.reader, Vocal::read_from)?;
        let (headers, textures, definitions) = if !self.vocals.is_empty() {
            let headers = read_vec(&mut decryptor.reader, SymbolsHeader::read_from)?;
            let textures = read_vec(&mut decryptor.reader, SymbolsTexture::read_from)?;
            let definitions = read_vec(&mut decryptor.reader, SymbolDefinition::read_from)?;
            (Some(headers), Some(textures), Some(definitions))
        } else {
            (None, None, None)
        };
        self.symbol_headers = headers;
        self.symbol_textures = textures;
        self.symbol_definitions = definitions;
        self.phrase_iterations = read_vec(&mut decryptor.reader, PhraseIteration::read_from)?;
        self.phrase_extra_info = read_vec(&mut decryptor.reader, PhraseExtraInfoByLevel::read_from)?;
        self.nld = read_vec(&mut decryptor.reader, NLinkedDifficulty::read_from)?;
        self.actions = read_vec(&mut decryptor.reader, Action::read_from)?;
        self.events = read_vec(&mut decryptor.reader, Event::read_from)?;
        self.tones = read_vec(&mut decryptor.reader, Tone::read_from)?;
        self.dnas = read_vec(&mut decryptor.reader, Dna::read_from)?;
        self.sections = read_vec(&mut decryptor.reader, Section::read_from)?;
        self.arrangements = read_vec(&mut decryptor.reader, Arrangement::read_from)?;
        self.metadata = Metadata::read_from(&mut decryptor.reader)?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct DIDX {
    pub wem_id: u32,
    pub offset: u32,
    pub length: u32,
}

#[derive(Default, Debug, Clone)]
pub struct BkhdAsset {
    pub bkhd_length: u32,
    pub bkhd_version: u32,
    pub bkhd_id: u32,
    pub didx_length: u32,
    pub didx: Vec<DIDX>,
    pub data_length: i32,
}


impl PsarcAsset for BkhdAsset {
    fn read_from<R: Read + Seek>(&mut self, reader: &mut R, _length: usize) -> io::Result<()> {
        let mut label_buf = [0u8; 4];
        reader.read_exact(&mut label_buf)?;
        let label = std::str::from_utf8(&label_buf)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid header label encoding"))?;
        if label != "BKHD" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not a valid bnk file"));
        }

        self.bkhd_length = reader.read_u32::<LittleEndian>()?;
        self.bkhd_version = reader.read_u32::<LittleEndian>()?;
        self.bkhd_id = reader.read_u32::<LittleEndian>()?;

        let mut cur = self.bkhd_length.checked_sub(8)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bkhd_length less than 8"))?;
        while cur > 0 {
            let _ = reader.read_i32::<LittleEndian>()?;
            cur = cur.saturating_sub(4);
        }
        let mut didx_label_buf = [0u8; 4];
        reader.read_exact(&mut didx_label_buf)?;
        let didx_label = std::str::from_utf8(&didx_label_buf)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid DIDX label encoding"))?;
        self.didx_length = reader.read_u32::<LittleEndian>()?;
        let mut cur = self.didx_length;
        let mut d_list = Vec::new();
        while cur > 0 {
            let wem_id = reader.read_u32::<LittleEndian>()?;
            let offset = reader.read_u32::<LittleEndian>()?;
            let length = reader.read_u32::<LittleEndian>()?;
            d_list.push(DIDX { wem_id, offset, length });
            cur = cur.saturating_sub(12);
        }
        self.didx = d_list;
        let mut data_label_buf = [0u8; 4];
        reader.read_exact(&mut data_label_buf)?;
        let data_label = std::str::from_utf8(&data_label_buf)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid DATA label encoding"))?;
        self.data_length = reader.read_i32::<LittleEndian>()?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct PsarcFile {
    pub header: PsarcFileHeader,
    pub toc: PsarcTOC,
    pub data: Vec<u8>,
}

impl PsarcFile {
    /// Opens the PSARC file from a reader. This method:
    /// 1. Reads the header.
    /// 2. Reads the TOC.
    /// 3. Seeks back to the start and reads the entire file into memory.
    pub fn open<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let header = PsarcFileHeader::read_from(reader)?;
        let toc = PsarcTOC::read_from(&mut *reader, &header)?;
        reader.seek(SeekFrom::Start(0))?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(PsarcFile { header, toc, data })
    }

    pub fn get_entry_by_file_name(&self, file_name: &str) -> Option<&PsarcTOCEntry> {
        self.toc.entries.iter().find(|entry| {
            if let Some(entry_path_str) = &entry.path {
                let entry_path = Path::new(entry_path_str);
                if let Some(entry_file_name) = entry_path.file_name() {
                    entry_file_name.to_string_lossy() == file_name
                } else {
                    false
                }
            } else {
                false
            }
        })
    }

    /// Inflates an entry into an asset of type T.
    /// This method creates a new cursor over the entire file data, then calls
    /// `inflate_entry_data` to perform block‑by‑block inflation of the specified entry.
    pub fn inflate_entry_as<T: PsarcAsset + Default>(&self, entry: &PsarcTOCEntry) -> io::Result<T> {
        let inflated = self.inflate_entry_data(entry)?;
        let mut asset = T::default();
        let mut cursor = Cursor::new(inflated);
        let cursor_length = cursor.get_ref().len();
        asset.read_from(&mut cursor, cursor_length)?;
        Ok(asset)
    }

    /// Performs block‑by‑block inflation (decompression) of the asset specified by `entry`.
    /// Returns a Vec<u8> containing the uncompressed asset data.
    pub fn inflate_entry_data(&self, entry: &PsarcTOCEntry) -> io::Result<Vec<u8>> {
        let block_size = self.header.block_size as usize;
        // Calculate how many blocks the uncompressed asset spans.
        let num_blocks = ((entry.length as f64) / (block_size as f64)).ceil() as u32;
        let last_block = entry.start_block + num_blocks - 1;
        
        // Create a cursor over the file data and seek to the asset's offset.
        let mut cursor = Cursor::new(&self.data);
        cursor.seek(SeekFrom::Start(entry.offset))?;
        
        let mut output = Vec::new();
        const ZIP_HEADER: u16 = 0x78DA;
        
        // For each block index from entry.start_block to last_block:
        for block in entry.start_block..=last_block {
            // Get the ZIP block size for this block.
            // (If the TOC does not provide a size for this block, assume 0.)
            let zipblock_size = self.toc.zip_block_sizes.get(block as usize).copied().unwrap_or(0) as usize;
            
            if zipblock_size == 0 {
                // Uncompressed: read a full block.
                let mut buf = vec![0u8; block_size];
                let n = cursor.read(&mut buf)?;
                output.extend_from_slice(&buf[..n]);
            } else {
                // Peek at the first two bytes.
                let pos = cursor.position();
                let header_val = cursor.read_u16::<BigEndian>()?;
                // Rewind 2 bytes.
                cursor.seek(SeekFrom::Start(pos))?;
                
                if header_val == ZIP_HEADER {
                    // Compressed block: call unzip_block.
                    let decompressed = unzip_block(&mut cursor, zipblock_size)?;
                    output.extend_from_slice(&decompressed);
                } else {
                    // Otherwise, read raw zipblock_size bytes.
                    let mut buf = vec![0u8; zipblock_size];
                    cursor.read_exact(&mut buf)?;
                    output.extend_from_slice(&buf);
                }
            }
        }
        // Truncate the output to exactly entry.length bytes.
        output.truncate(entry.length as usize);
        Ok(output)
    }

    /// Reads the manifest from TOC entry 0.
    /// Sets TOC.Entries[0].path to "NamesBlock.bin", inflates the entry as a TextPsarcAsset,
    /// and assigns each line as the path for subsequent TOC entries.
    pub fn read_manifest(&mut self) -> io::Result<()> {
        if self.toc.entries.is_empty() {
            return Ok(());
        }
        self.toc.entries[0].path = Some("NamesBlock.bin".to_string());
        let asset: TextAsset = self.inflate_entry_as(&self.toc.entries[0])?;
        tracing::trace!("Manifest text ({} bytes):", asset.text.len());
        tracing::trace!("{}", asset.text);
        for (i, line) in asset.lines.iter().enumerate() {
            self.toc.entries[i + 1].path = Some(line.to_string());
        }
        Ok(())
    }

    pub fn convert_sng_assets_to_json(&mut self, output_dir: &Path) -> io::Result<()> {
        if self.toc.entries.is_empty() {
            return Ok(());
        }

        for entry in &self.toc.entries {
            if let Some(ref path) = entry.path {
                if path.ends_with(".sng") {
                    let asset: SngAsset = self.inflate_entry_as(entry)?;
                    tracing::trace!(
                        "Converted SNG asset from {} (metadata: {:?})",
                        path,
                        asset.metadata
                    );
                    let json = serde_json::to_string_pretty(&asset)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                    let file_name = Path::new(path)
                        .file_name()
                        .expect("Entry path should have a file name");
                    let output_file_name = format!("{}.json", file_name.to_string_lossy());
                    let output_file_path = output_dir.join(output_file_name);
                    
                    fs::write(&output_file_path, json)?;
                    tracing::info!("Written JSON asset to {:?}", output_file_path);
                }
            }
        }
        Ok(())
    }

    pub fn dump_entries(&mut self, output_dir: &Path) -> io::Result<()> {
        fs::create_dir_all(output_dir)?;
        for entry in &self.toc.entries {
            if let Some(path) = &entry.path {
                tracing::trace!("Dumping entry: {}", path);
                let data = self.inflate_entry_data(entry)?;
                let output_path = output_dir.join(Path::new(path).file_name().unwrap());
                let mut file = fs::File::create(&output_path)?;
                file.write_all(&data)?;
                tracing::info!("Data dumped to {:?}", output_path);
            }
        }
        Ok(())
    }
}

/// Decompresses a block using Deflate.
///
/// This function mimics the C# UnzipBlock method by:
/// 1. Skipping the first 2 bytes (the header bytes).
/// 2. Reading the remaining bytes (size - 2) from the input.
/// 3. Decompressing the data using DeflateDecoder.
///
pub fn unzip_block<R: Read + Seek>(reader: &mut R, size: usize) -> io::Result<Vec<u8>> {
    reader.seek(SeekFrom::Current(2))?;
    let comp_size = size.checked_sub(2)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Size must be at least 2"))?;
    
    let mut comp_data = vec![0u8; comp_size];
    reader.read_exact(&mut comp_data)?;

    let mut decoder = DeflateDecoder::new(&comp_data[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    
    Ok(decompressed)
}