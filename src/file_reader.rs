use std::fs;
use std::io::{self, Cursor};

#[derive(Debug)]
pub struct MemFile {
    pub data: Vec<u8>,
}

impl MemFile {
    pub fn read_from_path(path: &str) -> io::Result<Self> {
        let data = fs::read(path)?;
        Ok(MemFile { data })
    }
    
    pub fn size(&self) -> usize {
        self.data.len()
    }
    
    pub fn as_cursor(&self) -> Cursor<&[u8]> {
        Cursor::new(&self.data)
    }
}