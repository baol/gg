//! Disk-backed rows shared by paged views.
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
};
const PAGE: usize = 256;
fn error(e: impl std::fmt::Display) -> String {
    format!("Could not store rows: {e}")
}
pub struct Store<T> {
    marker: std::marker::PhantomData<T>,
    data: File,
    index: File,
    pub count: usize,
}
impl<T: serde::Serialize + serde::de::DeserializeOwned> Store<T> {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            marker: std::marker::PhantomData,
            data: tempfile::tempfile().map_err(error)?,
            index: tempfile::tempfile().map_err(error)?,
            count: 0,
        })
    }
    pub fn append(&mut self, change: &T) -> Result<(), String> {
        let bytes = serde_json::to_vec(change).map_err(error)?;
        let offset = self.data.seek(SeekFrom::End(0)).map_err(error)?;
        self.data.write_all(&bytes).map_err(error)?;
        self.index.seek(SeekFrom::End(0)).map_err(error)?;
        self.index.write_all(&offset.to_le_bytes()).map_err(error)?;
        self.index
            .write_all(&(bytes.len() as u64).to_le_bytes())
            .map_err(error)?;
        self.count += 1;
        Ok(())
    }
    pub fn page(&mut self, range: Range<usize>) -> Result<Vec<(usize, T)>, String> {
        let mut rows = Vec::new();
        for row in range.start
            ..range
                .end
                .min(self.count)
                .min(range.start.saturating_add(PAGE))
        {
            self.index
                .seek(SeekFrom::Start(row as u64 * 16))
                .map_err(error)?;
            let (mut offset, mut len) = ([0; 8], [0; 8]);
            self.index.read_exact(&mut offset).map_err(error)?;
            self.index.read_exact(&mut len).map_err(error)?;
            self.data
                .seek(SeekFrom::Start(u64::from_le_bytes(offset)))
                .map_err(error)?;
            let mut bytes = vec![0; u64::from_le_bytes(len) as usize];
            self.data.read_exact(&mut bytes).map_err(error)?;
            rows.push((row, serde_json::from_slice(&bytes).map_err(error)?));
        }
        Ok(rows)
    }
}
