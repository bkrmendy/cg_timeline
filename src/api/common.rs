use blake2b_simd::blake2b;
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
) -> anyhow::Result<String> {
    conn.read_branch_tip(branch_name).and_then(|tip| {
        tip.ok_or(anyhow::Error::new(DBError::Error(
            "Branch tip does not exist".to_owned(),
        )))
    })
}

pub fn get_file_mod_time(file_path: &str) -> Result<i64, DBError> {
    let metadata = fs::metadata(file_path)
        .map_err(|e| DBError::Consistency(format!("File {} does not exist ({}))", file_path, e)))?;

    Ok(FileTime::from_last_modification_time(&metadata).unix_seconds())
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

pub fn get_hash(data: &[u8]) -> String {
    blake2b(data).to_hex().to_string()
}

pub fn blend_file_data_from_file(
    path_to_blend: &str,
) -> Result<BlendFileDataForCheckpoint, String> {
    let exists = std::path::Path::new(path_to_blend).exists();
    if !exists {
        return Err(String::from("Blend file does not exist"));
    }
    let blend_bytes = measure_time!(format!("Reading {:?}", path_to_blend), {
        from_file(path_to_blend).map_err(|e| format!("Cannot read blend file: {}", e))
    })?;

    let parsed_blend = measure_time!(format!("Parsing blocks {:?}", path_to_blend), {
        parse_blend(blend_bytes).unwrap()
    });

    let endianness = parsed_blend.header.endianness;

    println!("Number of blocks: {:?}", parsed_blend.blocks.len());

    let block_data_with_meta: Vec<(BlockMetadata, Vec<u8>)> = measure_time!(
        format!("Hashing and compressing blocks {:?}", path_to_blend),
        {
            parsed_blend
                .blocks
                .into_par_iter()
                .map(|parsed_block| {
                    let mut block_blob: Vec<u8> = vec![];
                    print_block_manual(parsed_block.simple_block, endianness, &mut block_blob);

                    let hash = get_hash(&block_blob);

                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    encoder
                        .write_all(&block_blob)
                        .map_err(|e| format!("Cannot encode: {:?}", e))?;
                    let compressed = encoder
                        .finish()
                        .map_err(|e| format!("Cannot encode: {:?}", e))?;

                    Ok((
                        BlockMetadata {
                            hash,
                            original_mem_address: parsed_block.original_mem_address,
                            pointers: parsed_block.pointers,
                        },
                        compressed,
                    ))
                })
                .collect::<Vec<Result<(BlockMetadata, Vec<u8>), String>>>()
                .into_iter()
                .collect::<Result<Vec<(BlockMetadata, Vec<u8>)>, String>>()
        }
    )?;

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
        get_hash(&block_meta_bytes)
    });

    Ok(BlendFileDataForCheckpoint {
        hash: blend_hash,
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
        parse_print_blend::{parse_blend, print_blend},
        utils::from_file,
    };

    #[test]
    fn test_parse_print_blend() {
        let blend_bytes = from_file("data/fixtures/untitled.blend").unwrap();
        let blend_m = parse_blend(blend_bytes).unwrap();
        let blend_mm = blend_m.clone();

        assert_eq!(blend_m.header.endianness, Endianness::Little);
        assert_eq!(blend_m.header.pointer_size, PointerSize::Bits64);
        assert_eq!(blend_m.header.version, [51, 48, 51]);
        insta::assert_debug_snapshot!(blend_m.blocks.len(), @"2159");

        let mut blend_bytes_m: Vec<u8> = vec![];
        print_blend(blend_m, &mut blend_bytes_m);

        let blend_again = parse_blend(blend_bytes_m).unwrap();

        assert_eq!(blend_again.header.endianness, blend_mm.header.endianness);
        assert_eq!(
            blend_again.header.pointer_size,
            blend_mm.header.pointer_size
        );
        assert_eq!(blend_again.header.version, blend_mm.header.version);
        assert_eq!(blend_again.blocks.len(), blend_mm.blocks.len());
    }
}
