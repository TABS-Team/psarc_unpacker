use std::process;
use std::io::{Cursor};
use std::fs;
use std::path::Path;

use psarc_unpacker::psarc::PsarcFileHeader;
use psarc_unpacker::psarc::PsarcTOC;
use psarc_unpacker::psarc::PsarcFile;
use psarc_unpacker::file_reader::MemFile;


fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let file_path = "mop.psarc";
    let output_folder = Path::new("mop");

    let mem_file = fs::read(file_path)?;
    let mut cursor =  Cursor::new(&mem_file);

    println!("Successfully read file: {}", file_path);
    println!("File size: {} bytes", mem_file.len());

    let mut psarc_file = PsarcFile::open(&mut cursor)?;
    psarc_file.read_manifest()?;

    // Iterate over the TOC entries and print each entry's path.
    for (i, entry) in psarc_file.toc.entries.iter().enumerate() {
        println!("Entry {} path: {:?}", i, entry.path);
    }
    psarc_file.dump_entries(output_folder)?;
    Ok(())
}