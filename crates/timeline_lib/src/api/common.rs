use filetime::FileTime;
use flate2::{write::GzEncoder, Compression};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    blend::{
        parse_print_blend::{
            parse_blend, print_block_manual, print_header_manual, OffsetsWithPointerValue,
        },
        utils::{from_file, Either},
    },
    db::{
        db_ops::{DBError, Persistence, DB},
        structs::BlockRecord,
    },
    measure_time,
};

use std::{fs, io::Write};

pub fn read_latest_commit_hash_on_branch(
    conn: &Persistence,
    branch_name: &str,
) -> Result<String, DBError> {
    conn.read_branch_tip(branch_name)
        .and_then(|tip| tip.ok_or(DBError::Error("Branch tip does not exist".to_owned())))
}

pub fn get_file_mod_time(file_path: &str) -> Result<i64, DBError> {
    let metadata = fs::metadata(file_path)
        .map_err(|e| DBError::Consistency(format!("File {} does not exist ({}))", file_path, e)))?;

    Ok(FileTime::from_last_modification_time(&metadata).unix_seconds())
}

pub fn check_if_file_modified(db: &Persistence, file_path: &str) -> Result<i64, DBError> {
    let last_mod_time_from_file = get_file_mod_time(file_path)?;
    let last_mod_time_from_db = db.read_last_modification_time()?;

    if last_mod_time_from_db.is_none() {
        return Ok(last_mod_time_from_file);
    }

    let last_mod_time_from_db = last_mod_time_from_db.unwrap();

    if last_mod_time_from_db <= last_mod_time_from_file {
        return Ok(last_mod_time_from_file);
    }

    Err(DBError::Error(
        "File not modified since the last change".to_owned(),
    ))
}

pub struct BlendFileDataForCheckpoint {
    pub hash: String,
    pub header_bytes: Vec<u8>,
    pub blocks_and_pointers_bytes: Vec<u8>,
    pub block_data: Vec<BlockRecord>,
}

#[derive(Serialize, Deserialize)]
pub struct BlockMetadata {
    pub hash: String,
    pub original_mem_address: Either<u32, u64>,
    pub pointers: OffsetsWithPointerValue, // offset with pointer value
}

pub fn blend_file_data_from_file(
    path_to_blend: &str,
) -> Result<BlendFileDataForCheckpoint, String> {
    let blend_bytes = measure_time!(format!("Reading {:?}", path_to_blend), {
        from_file(path_to_blend).map_err(|_| "Cannot unpack blend file".to_owned())
    })?;

    let parsed_blend = measure_time!(format!("Parsing blocks {:?}", path_to_blend), {
        parse_blend(blend_bytes).unwrap()
    });

    let endianness = parsed_blend.header.endianness;

    println!("Number of blocks: {:?}", parsed_blend.blocks.len());

    let block_data_with_meta: Vec<(BlockMetadata, Vec<u8>)> =
        measure_time!(format!("Hashing blocks {:?}", path_to_blend), {
            parsed_blend
                .blocks
                .into_par_iter()
                .map(|parsed_block| {
                    let mut block_blob: Vec<u8> = vec![];
                    print_block_manual(parsed_block.simple_block, endianness, &mut block_blob);

                    let hash = md5::compute(&block_blob);

                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    encoder
                        .write_all(&block_blob)
                        .map_err(|e| format!("Cannot encode: {:?}", e))?;
                    let compressed = encoder
                        .finish()
                        .map_err(|e| format!("Cannot encode: {:?}", e))?;

                    Ok((
                        BlockMetadata {
                            hash: format!("{:x}", hash),
                            original_mem_address: parsed_block.original_mem_address,
                            pointers: parsed_block.pointers,
                        },
                        compressed,
                    ))
                })
                .collect::<Vec<Result<(BlockMetadata, Vec<u8>), String>>>()
                .into_iter()
                .collect::<Result<Vec<(BlockMetadata, Vec<u8>)>, String>>()
        })?;

    let mut header_data: Vec<u8> = vec![];
    print_header_manual(parsed_blend.header, &mut header_data);

    let block_records: Vec<BlockRecord> = block_data_with_meta
        .par_iter()
        .map(|(meta, data)| BlockRecord {
            hash: meta.hash.clone(),
            data: data.to_owned(),
        })
        .collect();

    let blocks_meta: Vec<BlockMetadata> = block_data_with_meta
        .into_iter()
        .map(|(meta, _)| meta)
        .collect();

    let block_meta_bytes = print_blocks_and_pointers(blocks_meta);

    let blend_hash = measure_time!(format!("Hashing {:?}", path_to_blend), {
        md5::compute(&block_meta_bytes)
    });

    Ok(BlendFileDataForCheckpoint {
        hash: format!("{:x}", blend_hash),
        header_bytes: header_data,
        blocks_and_pointers_bytes: block_meta_bytes,
        block_data: block_records,
    })
}

pub fn print_blocks_and_pointers(data: Vec<BlockMetadata>) -> Vec<u8> {
    bincode::serialize(&data).unwrap()
}

pub fn parse_blocks_and_pointers(data: &[u8]) -> Vec<BlockMetadata> {
    bincode::deserialize(data).unwrap()
}

#[cfg(test)]
mod test {
    use crate::blend::{
        blend_file::{Endianness, PointerSize},
        parse_print_blend::print_blend_manual,
        utils::from_file,
    };

    // #[ignore]
    // #[test]
    // fn test_parse_print_blend() {
    //     let blend_bytes = from_file("data/fixtures/untitled.blend").unwrap();
    //     let blend_m = parse_blend_manual(blend_bytes).unwrap();
    //     let blend_mm = blend_m.clone();

    //     assert_eq!(blend_m.header.endianness, Endianness::Little);
    //     assert_eq!(blend_m.header.pointer_size, PointerSize::Bits64);
    //     assert_eq!(blend_m.header.version, [51, 48, 51]);
    //     assert_eq!(blend_m.blocks.len(), 2159);

    //     let mut blend_bytes_m: Vec<u8> = vec![];
    //     print_blend_manual(blend_m, &mut blend_bytes_m);

    //     let blend_again = parse_blend_manual(blend_bytes_m).unwrap();

    //     assert_eq!(blend_again.header.endianness, blend_mm.header.endianness);
    //     assert_eq!(
    //         blend_again.header.pointer_size,
    //         blend_mm.header.pointer_size
    //     );
    //     assert_eq!(blend_again.header.version, blend_mm.header.version);
    //     assert_eq!(blend_again.blocks.len(), blend_mm.blocks.len());
    // }
}
