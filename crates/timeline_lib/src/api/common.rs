use filetime::FileTime;
use flate2::{write::GzEncoder, Compression};
use rayon::prelude::*;

use crate::{
    blend::{
        blend_file::{
            DNAField, DNAInfo, DNAStruct, Endianness, Header, PointerSize, SimpleParsedBlock,
        },
        utils::{from_file, Either},
    },
    db::{
        db_ops::{DBError, Persistence, DB},
        structs::BlockRecord,
    },
    measure_time,
};

use std::{fmt::Display, fs, io::Write};

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

    if last_mod_time_from_db < last_mod_time_from_file {
        return Ok(last_mod_time_from_file);
    }

    Err(DBError::Error(
        "File not modified since the last change".to_owned(),
    ))
}

pub struct BlendFileDataForCheckpoint {
    pub hash: String,
    pub header_bytes: Vec<u8>,
    pub blocks: String,
    pub block_data: Vec<BlockRecord>,
}

pub fn blend_file_data_from_file(
    path_to_blend: &str,
) -> Result<BlendFileDataForCheckpoint, String> {
    let blend_bytes = measure_time!(format!("Reading {:?}", path_to_blend), {
        from_file(path_to_blend).map_err(|_| "Cannot unpack blend file".to_owned())
    })?;

    let parsed_blend = measure_time!(format!("Parsing blocks {:?}", path_to_blend), {
        parse_blend_manual(blend_bytes).unwrap()
    });

    let endianness = parsed_blend.header.endianness;

    println!("Number of blocks: {:?}", parsed_blend.blocks.len());

    let block_records: Vec<BlockRecord> =
        measure_time!(format!("Hashing blocks {:?}", path_to_blend), {
            parsed_blend
                .blocks
                .into_par_iter()
                .map(|parsed_block| {
                    let mut block_blob: Vec<u8> = vec![];
                    print_block_manual(parsed_block, endianness, &mut block_blob);

                    let hash = md5::compute(&block_blob);

                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    encoder
                        .write_all(&block_blob)
                        .map_err(|e| format!("Cannot encode: {:?}", e))?;
                    let compressed = encoder
                        .finish()
                        .map_err(|e| format!("Cannot encode: {:?}", e))?;

                    Ok(BlockRecord {
                        hash: format!("{:x}", hash),
                        data: compressed,
                    })
                })
                .collect::<Vec<Result<BlockRecord, String>>>()
                .into_iter()
                .collect::<Result<Vec<BlockRecord>, String>>()
        })?;

    let mut header_data: Vec<u8> = vec![];
    print_header_manual(parsed_blend.header, &mut header_data);
    let block_hashes: Vec<String> = measure_time!("Collecting block hashes", {
        block_records
            .iter()
            .map(move |b| b.hash.to_owned())
            .collect()
    });
    let blocks_str = measure_time!("Printing hash list", print_hash_list(block_hashes));

    let blend_hash = measure_time!(format!("Hashing {:?}", path_to_blend), {
        md5::compute(&blocks_str)
    });

    Ok(BlendFileDataForCheckpoint {
        hash: format!("{:x}", blend_hash),
        header_bytes: header_data,
        blocks: blocks_str,
        block_data: block_records,
    })
}

#[derive(Debug, Clone)]
pub struct ParsedBlendFile {
    pub header: Header,
    pub blocks: Vec<SimpleParsedBlock>,
}

#[derive(Debug)]
pub enum BlendFileParseError {
    NotABlendFile,
    UnexpectedEndOfInput,
    ConversionFailed,
    TagNotMatching(String),
}

impl Display for BlendFileParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlendFileParseError::NotABlendFile => write!(f, "Not a blend file"),
            BlendFileParseError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            BlendFileParseError::ConversionFailed => write!(f, "Conversion failed"),
            BlendFileParseError::TagNotMatching(tag) => write!(f, "Tag did not match: {}", tag),
        }
    }
}

fn lmap<L, R, LL, F>(t: (L, R), f: F) -> (LL, R)
where
    F: FnOnce(L) -> LL,
{
    let (l, r) = t;
    (f(l), r)
}

type BlendFileParseResult<'a, T> = Result<(T, &'a [u8]), BlendFileParseError>;

fn get_bytes<const N: usize>(data: &[u8]) -> BlendFileParseResult<[u8; N]> {
    let bytes: [u8; N] = data[0..N]
        .try_into()
        .map_err(|_| BlendFileParseError::UnexpectedEndOfInput)?;

    let rest = &data[N..];
    Ok((bytes, rest))
}

fn get_tag<const N: usize>(data: &[u8], tag: [u8; N]) -> BlendFileParseResult<()> {
    let (result, data) = get_bytes::<N>(data)?;
    if result == tag {
        return Ok(((), data));
    }

    Err(BlendFileParseError::TagNotMatching(
        String::from_utf8(tag.to_vec()).unwrap(),
    ))
}

fn multi<T, F>(data: &[u8], times: i32, callback: F) -> BlendFileParseResult<Vec<T>>
where
    F: Fn(&[u8]) -> BlendFileParseResult<T>,
{
    let mut working = data;
    let mut result: Vec<T> = Vec::new();
    for _ in 0..times {
        let (parsed, rest_of_data) = callback(working)?;
        working = rest_of_data;
        result.push(parsed)
    }

    Ok((result, working))
}

fn parse_u16(data: &[u8], endianness: Endianness) -> BlendFileParseResult<u16> {
    match endianness {
        Endianness::Big => get_bytes::<2>(data).map(|bs| lmap(bs, u16::from_be_bytes)),
        Endianness::Little => get_bytes::<2>(data).map(|bs| lmap(bs, u16::from_le_bytes)),
    }
}

fn parse_u32(data: &[u8], endianness: Endianness) -> BlendFileParseResult<u32> {
    match endianness {
        Endianness::Big => get_bytes::<4>(data).map(|bs| lmap(bs, u32::from_be_bytes)),
        Endianness::Little => get_bytes::<4>(data).map(|bs| lmap(bs, u32::from_le_bytes)),
    }
}

fn parse_i16(data: &[u8], endianness: Endianness) -> BlendFileParseResult<i16> {
    match endianness {
        Endianness::Big => get_bytes::<2>(data).map(|bs| lmap(bs, i16::from_be_bytes)),
        Endianness::Little => get_bytes::<2>(data).map(|bs| lmap(bs, i16::from_le_bytes)),
    }
}

fn parse_i32(data: &[u8], endianness: Endianness) -> BlendFileParseResult<i32> {
    match endianness {
        Endianness::Big => get_bytes::<4>(data).map(|bs| lmap(bs, i32::from_be_bytes)),
        Endianness::Little => get_bytes::<4>(data).map(|bs| lmap(bs, i32::from_le_bytes)),
    }
}

fn parse_null_terminated_string(data: &[u8]) -> BlendFileParseResult<String> {
    let null_range_end = data
        .iter()
        .position(|&c| c == b'\0')
        .ok_or(BlendFileParseError::UnexpectedEndOfInput)?;

    let string = String::from_utf8(data[0..null_range_end].to_vec())
        .map_err(|_| BlendFileParseError::ConversionFailed)?;
    let rest = &data[null_range_end + 1..];

    Ok((string, rest))
}

fn parse_padding_zeros(data: &[u8]) -> BlendFileParseResult<&[u8]> {
    let padding_end = data
        .iter()
        .position(|&c| c != b'\0')
        .ok_or(BlendFileParseError::UnexpectedEndOfInput)?;

    Ok((&data[0..padding_end], &data[padding_end..]))
}

fn get_byte_vec(data: &[u8], count: u32) -> BlendFileParseResult<Vec<u8>> {
    let count: usize = count
        .try_into()
        .map_err(|_| BlendFileParseError::UnexpectedEndOfInput)?;
    if data.len() < count {
        return Err(BlendFileParseError::UnexpectedEndOfInput);
    }

    Ok((data[0..count].to_vec(), &data[count..]))
}

fn parse_u64(data: &[u8], endianness: Endianness) -> BlendFileParseResult<u64> {
    match endianness {
        Endianness::Big => get_bytes::<8>(data).map(|bs| lmap(bs, u64::from_be_bytes)),
        Endianness::Little => get_bytes::<8>(data).map(|bs| lmap(bs, u64::from_le_bytes)),
    }
}

pub fn parse_header_manual(data: &[u8]) -> BlendFileParseResult<Header> {
    let (header, rest) = get_bytes::<7>(data)?;
    if &header != b"BLENDER" {
        return Err(BlendFileParseError::NotABlendFile);
    }

    let (pointer_size, rest) = get_bytes::<1>(rest).map(|res| {
        lmap(res, |[p]| {
            if p == b'_' {
                PointerSize::Bits32
            } else {
                PointerSize::Bits64
            }
        })
    })?;

    let (endianness, rest) = get_bytes::<1>(rest).map(|res| {
        lmap(res, |[p]| {
            if p == b'v' {
                Endianness::Little
            } else {
                Endianness::Big
            }
        })
    })?;

    let (version, rest) = get_bytes::<3>(rest)?;

    Ok((
        Header {
            pointer_size,
            endianness,
            version,
        },
        rest,
    ))
}

pub fn print_header_manual(header: Header, out: &mut Vec<u8>) {
    out.extend(b"BLENDER");
    let pointer_size = match header.pointer_size {
        PointerSize::Bits32 => b'_',
        PointerSize::Bits64 => b'-',
    };
    out.push(pointer_size);
    let endianness = match header.endianness {
        Endianness::Little => b'v',
        Endianness::Big => b'V',
    };
    out.push(endianness);
    out.extend(header.version)
}

fn print_u32(value: u32, endianness: Endianness, out: &mut Vec<u8>) {
    let bytes = match endianness {
        Endianness::Little => value.to_le_bytes(),
        Endianness::Big => value.to_be_bytes(),
    };
    out.extend(bytes)
}

fn print_u64(value: u64, endianness: Endianness, out: &mut Vec<u8>) {
    let bytes = match endianness {
        Endianness::Little => value.to_le_bytes(),
        Endianness::Big => value.to_be_bytes(),
    };
    out.extend(bytes)
}

pub fn parse_block_manual(
    blend_data: &[u8],
    pointer_size: PointerSize,
    endianness: Endianness,
) -> BlendFileParseResult<SimpleParsedBlock> {
    let (code, blend_data) = get_bytes::<4>(blend_data)?;
    let (size, blend_data) = parse_u32(blend_data, endianness)?;
    let (memory_address, blend_data) = match pointer_size {
        PointerSize::Bits32 => {
            parse_u32(blend_data, endianness).map(|res| lmap(res, Either::Left))?
        }
        PointerSize::Bits64 => {
            parse_u64(blend_data, endianness).map(|res| lmap(res, Either::Right))?
        }
    };

    let (dna_index, blend_data) = parse_u32(blend_data, endianness)?;
    let (count, blend_data) = parse_u32(blend_data, endianness)?;
    let (data, blend_data) = get_byte_vec(blend_data, size)?;

    Ok((
        SimpleParsedBlock {
            code,
            size,
            memory_address,
            dna_index,
            count,
            data,
        },
        blend_data,
    ))
}

fn parse_field(data: &[u8], endianness: Endianness) -> BlendFileParseResult<DNAField> {
    let (type_idx, data) = parse_i16(data, endianness)?;
    let (name_idx, data) = parse_i16(data, endianness)?;
    Ok((DNAField { type_idx, name_idx }, data))
}

fn parse_struct(data: &[u8], endianness: Endianness) -> BlendFileParseResult<DNAStruct> {
    let (type_idx, data) = parse_i16(data, endianness)?;
    let (fields_len, data) = parse_i16(data, endianness)?;
    let fields_len_as_i32 = i32::from(fields_len);
    let (fields, data) = multi(data, fields_len_as_i32, |d| parse_field(d, endianness))?;

    Ok((DNAStruct { type_idx, fields }, data))
}

pub fn parse_sdna(data: &[u8], endianness: Endianness) -> BlendFileParseResult<DNAInfo> {
    let ((), data) = get_tag(data, *b"SDNA")?;

    let ((), data) = get_tag(data, *b"NAME")?;
    let (names_len, data) = parse_i32(data, endianness)?;
    let (names, data) = multi(data, names_len, parse_null_terminated_string)?;

    let (_, data) = parse_padding_zeros(data)?;

    let ((), data) = get_tag(data, *b"TYPE")?;
    let (types_len, data) = parse_i32(data, endianness)?;
    let (types, data) = multi(data, types_len, parse_null_terminated_string)?;

    let (_, data) = parse_padding_zeros(data)?;

    let ((), data) = get_tag::<4>(data, *b"TLEN")?;
    let (type_lengths, data) = multi(data, types_len, |d| parse_u16(d, endianness))?;

    let (_, data) = parse_padding_zeros(data)?;

    let ((), data) = get_tag(data, *b"STRC")?;
    let (structs_len, data) = parse_i32(data, endianness)?;
    let (structs, data) = multi(data, structs_len, |d| parse_struct(d, endianness))?;

    Ok((
        DNAInfo {
            names,
            types,
            type_lengths,
            structs,
        },
        data,
    ))
}

pub fn print_block_manual(block: SimpleParsedBlock, endianness: Endianness, out: &mut Vec<u8>) {
    out.extend(block.code);
    print_u32(block.size, endianness, out);
    match block.memory_address {
        Either::Left(mem) => print_u32(mem, endianness, out),
        Either::Right(mem) => print_u64(mem, endianness, out),
    };
    print_u32(block.dna_index, endianness, out);
    print_u32(block.count, endianness, out);
    out.extend(block.data);
}

pub fn parse_blend_manual(blend_data: Vec<u8>) -> Result<ParsedBlendFile, BlendFileParseError> {
    let (header, mut data) = parse_header_manual(&blend_data)?;
    let mut blocks: Vec<SimpleParsedBlock> = vec![];

    while !data.starts_with(b"ENDB") {
        let (next_block, rest_of_data) =
            parse_block_manual(data, header.pointer_size, header.endianness)?;
        blocks.push(next_block);

        data = rest_of_data;
    }
    Ok(ParsedBlendFile { header, blocks })
}

pub fn print_blend_manual(blend: ParsedBlendFile, out: &mut Vec<u8>) {
    let endianness = blend.header.endianness;
    print_header_manual(blend.header, out);
    for block in blend.blocks {
        print_block_manual(block, endianness, out)
    }
    out.extend(b"ENDB")
}

#[cfg(test)]
mod test {
    use crate::{
        api::common::print_blend_manual,
        blend::{
            blend_file::{Endianness, PointerSize},
            utils::from_file,
        },
    };

    use super::parse_blend_manual;

    #[test]
    fn test_parse_print_blend() {
        let blend_bytes = from_file("data/untitled.blend").unwrap();
        let blend_m = parse_blend_manual(blend_bytes).unwrap();
        let blend_mm = blend_m.clone();

        assert_eq!(blend_m.header.endianness, Endianness::Little);
        assert_eq!(blend_m.header.pointer_size, PointerSize::Bits64);
        assert_eq!(blend_m.header.version, [51, 48, 51]);
        assert_eq!(blend_m.blocks.len(), 1937);

        let mut blend_bytes_m: Vec<u8> = vec![];
        print_blend_manual(blend_m, &mut blend_bytes_m);

        let blend_again = parse_blend_manual(blend_bytes_m).unwrap();

        assert_eq!(blend_again.header.endianness, blend_mm.header.endianness);
        assert_eq!(
            blend_again.header.pointer_size,
            blend_mm.header.pointer_size
        );
        assert_eq!(blend_again.header.version, blend_mm.header.version);
        assert_eq!(blend_again.blocks.len(), blend_mm.blocks.len());
    }
}

pub fn parse_hash_list(raw: String) -> Vec<String> {
    raw.split(',').map(|s| s.to_string()).collect()
}

pub fn print_hash_list(raw: Vec<String>) -> String {
    raw.join(",")
}
