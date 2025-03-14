mod psarc;
mod decryptor;

use psarc::PsarcFileHeader;
use psarc::PsarcTOC;
use psarc::PsarcFile;

#[derive(Debug)]
pub struct TabsArrangement{
    
}

#[derive(Debug)]
pub struct TabsSong{
    pub song_name: string,
    pub arrangements: Vec<TabsArrangement>,
}