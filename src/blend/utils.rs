use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Cursor, Error, ErrorKind, Read, Write};
use tempfile::NamedTempFile;
use zstd::decode_all;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Either<Left, Right> {
    Left(Left),
    Right(Right),
}

fn decode_gzip(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let mut decoder = GzDecoder::new(bytes);
    let mut gzip_data = Vec::new();
    decoder.read_to_end(&mut gzip_data)?;

    Ok(gzip_data)
}

fn decode_zstd(bytes: &Vec<u8>) -> Result<Vec<u8>, Error> {
    let mut reader = Cursor::new(bytes);
    decode_all(&mut reader)
}

pub fn from_file(path: &str) -> Result<Vec<u8>, Error> {
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    if data[0..7] != *b"BLENDER" {
        let unzipped = decode_gzip(&data).or(decode_zstd(&data)).map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Blend file not correctly encoded: {:?}", e),
            )
        })?;

        data = unzipped;
    }

    Ok(data)
}

pub fn to_file_transactional(
    path: &str,
    blend_data: Vec<u8>,
    terminator: Vec<u8>,
) -> Result<(), Error> {
    let temp_file = NamedTempFile::new()?;

    let mut gz = GzEncoder::new(&temp_file, Compression::default());
    gz.write_all(&blend_data)?;

    gz.write_all(&terminator)?;

    gz.flush()?;
    gz.finish()?;

    temp_file.persist(path)?;

    Ok(())
}
