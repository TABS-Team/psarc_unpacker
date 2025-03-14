use std::env;
use std::process;
mod file_reader;
mod psarc;
mod decryptor;

use file_reader::MemFile;
use psarc::PsarcFileHeader;
use psarc::PsarcTOC;
use psarc::PsarcFile;



// temp shit until I know what to do with all this
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::fs::{self,File};
use std::path::{Path, PathBuf};
use std::process::Command;
use image_dds::ddsfile::Dds;
use image_dds::image_from_dds;
use image;
use wem_converter::wwriff::{WwiseRiffVorbis, ForcePacketFormat};
use wem_converter::errors::{ParseError, Result};


fn convert_dds_to_png(dds_data: &[u8]) -> io::Result<Vec<u8>> {
    let dds = Dds::read(dds_data)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let dyn_img = image_from_dds(&dds, 0)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    let mut png_bytes = Vec::new();
    {
        let mut cursor = Cursor::new(&mut png_bytes);
        dyn_img.write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    }
    
    Ok(png_bytes)
}

fn dump_entries(psarc: &psarc::PsarcFile, output_dir: &Path) -> Result<()> {
    // Ensure the output directory exists.
    fs::create_dir_all(output_dir)?;
    
    for entry in &psarc.toc.entries {
        if let Some(path) = &entry.path {
            let filename = Path::new(path)
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("unknown"));
            
            // Dump audio files (.wem -> .ogg)
            if path.contains("audio/windows") && path.ends_with(".wem") {
                println!("Dumping audio: {}", path);
                let data = psarc.inflate_entry_data(entry)?;
                let output_path = output_dir.join(Path::new(path).file_name().unwrap()).with_extension("wem");
                let mut file = File::create(&output_path)?;
                file.write_all(&data)?;
                let mut codebooks_file = "bin/packed_codebooks.bin";
                let output_path_str = output_path.to_str()
                    .ok_or_else(|| ParseError::Message("Invalid output path".into()))?;

                let mut vorbis = WwiseRiffVorbis::new(
                    output_path_str,
                    codebooks_file,
                    false,
                    false,
                    ForcePacketFormat::ModPackets,
                )?;
                vorbis.generate_ogg()?;
                fs::remove_file(&output_path);
                println!("Ogg dumped to {:?}", output_dir.join(Path::new(path).file_name().unwrap()).with_extension("ogg"));
            }
            // Dump album art (.dds -> .png)
            else if path.contains("gfxassets/album_art") && path.ends_with(".dds") {
                println!("Dumping album art: {}", path);
                let data = psarc.inflate_entry_data(entry)?;
                let png_data = convert_dds_to_png(&data)?;
                let output_path = output_dir.join(Path::new(path).file_name().unwrap()).with_extension("png");
                let mut file = File::create(&output_path)?;
                file.write_all(&png_data)?;
                println!("PNG dumped to {:?}", output_path);
            }
            // Dump JSON files (just dump the raw JSON)
            else if path.contains("manifests") && path.ends_with(".json") {
                println!("Dumping JSON manifest: {}", path);
                let data = psarc.inflate_entry_data(entry)?;
                let output_path = output_dir.join(Path::new(path).file_name().unwrap()).with_extension("json");
                let mut file = File::create(&output_path)?;
                file.write_all(&data)?;
                println!("JSON dumped to {:?}", output_path);
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        process::exit(1);
    }
    let file_path = &args[1];
    let output_folder = Path::new(&args[2]);

    let mem_file = MemFile::read_from_path(file_path)?;

    println!("Successfully read file: {}", file_path);
    println!("File size: {} bytes", mem_file.size());

    let mut cursor = mem_file.as_cursor();
    let mut psarc_file = PsarcFile::open(&mut cursor)?;
    psarc_file.read_manifest()?;

    // Iterate over the TOC entries and print each entry's path.
    for (i, entry) in psarc_file.toc.entries.iter().enumerate() {
        println!("Entry {} path: {:?}", i, entry.path);
    }

    dump_entries(&psarc_file, output_folder)?;

    Ok(())
}