use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecureDeleteError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("wipe IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct SecureDelete;

impl SecureDelete {
    pub fn wipe_file(path: &Path) -> Result<(), SecureDeleteError> {
        if !path.exists() {
            return Err(SecureDeleteError::NotFound(
                path.display().to_string(),
            ));
        }

        let file_len = std::fs::metadata(path)?.len() as usize;
        let mut file = std::fs::OpenOptions::new().write(true).open(path)?;

        Self::dod_pass(&mut file, file_len, 0x00)?;
        Self::dod_pass(&mut file, file_len, 0xFF)?;
        Self::dod_pass(&mut file, file_len, 0x00)?;
        Self::random_pass(&mut file, file_len)?;
        Self::random_pass(&mut file, file_len)?;
        Self::random_pass(&mut file, file_len)?;
        Self::dod_pass(&mut file, file_len, 0x00)?;

        file.flush()?;
        drop(file);
        std::fs::remove_file(path)?;
        Ok(())
    }

    pub fn wipe_buffer(buf: &mut [u8]) {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        for byte in buf.iter_mut() { *byte = 0x00; }
        rng.fill_bytes(buf);
        for byte in buf.iter_mut() { *byte = 0x00; }
    }

    fn dod_pass(
        file: &mut std::fs::File,
        len: usize,
        fill: u8,
    ) -> Result<(), SecureDeleteError> {
        file.seek(SeekFrom::Start(0))?;
        let chunk = vec![fill; 4096.min(len)];
        let mut written = 0;
        while written < len {
            let to_write = (len - written).min(4096);
            file.write_all(&chunk[..to_write])?;
            written += to_write;
        }
        file.flush()?;
        Ok(())
    }

    fn random_pass(
        file: &mut std::fs::File,
        len: usize,
    ) -> Result<(), SecureDeleteError> {
        use rand::RngCore;
        file.seek(SeekFrom::Start(0))?;
        let mut rng = rand::thread_rng();
        let mut buf = vec![0u8; 4096.min(len)];
        let mut written = 0;
        while written < len {
            let to_write = (len - written).min(4096);
            rng.fill_bytes(&mut buf[..to_write]);
            file.write_all(&buf[..to_write])?;
            written += to_write;
        }
        file.flush()?;
        Ok(())
    }
}
