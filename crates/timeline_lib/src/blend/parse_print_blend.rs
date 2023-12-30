use std::{collections::HashMap, fmt::Display};

use rayon::iter::{
    IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use regex::Regex;

use crate::measure_time;

use super::{
    blend_file::{
        DNAField, DNAInfo, DNAStruct, Endianness, Header, PointerSize, SimpleParsedBlock,
    },
    utils::Either,
};

#[derive(Debug, Clone)]
pub struct ParsedBlendFile {
    pub header: Header,
    pub blocks: Vec<SimpleParsedBlock>,
}

#[derive(Debug)]
pub enum BlendFileParseError {
    NotABlendFile,
    UnexpectedEndOfInput(String),
    ConversionFailed,
    TagNotMatching(String),
}

impl Display for BlendFileParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlendFileParseError::NotABlendFile => write!(f, "Not a blend file"),
            BlendFileParseError::UnexpectedEndOfInput(source) => {
                write!(f, "Unexpected end of input: {}", source)
            }
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
        .map_err(|_| BlendFileParseError::UnexpectedEndOfInput("get_bytes".to_string()))?;

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
    let null_range_end =
        data.iter()
            .position(|&c| c == b'\0')
            .ok_or(BlendFileParseError::UnexpectedEndOfInput(
                "parse_null_terminated_string".to_string(),
            ))?;

    let string = String::from_utf8(data[0..null_range_end].to_vec())
        .map_err(|_| BlendFileParseError::ConversionFailed)?;
    let rest = &data[null_range_end + 1..];

    Ok((string, rest))
}

fn parse_padding_zeros(data: &[u8]) -> BlendFileParseResult<&[u8]> {
    let padding_end =
        data.iter()
            .position(|&c| c != b'\0')
            .ok_or(BlendFileParseError::UnexpectedEndOfInput(
                "parse_padding_zeros".to_string(),
            ))?;

    Ok((&data[0..padding_end], &data[padding_end..]))
}

fn get_byte_vec(data: &[u8], count: usize) -> BlendFileParseResult<Vec<u8>> {
    if data.len() < count {
        return Err(BlendFileParseError::UnexpectedEndOfInput(
            "get_byte_vec".to_string(),
        ));
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

fn print_i32(value: i32, endianness: Endianness, out: &mut Vec<u8>) {
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
    block_data: &[u8],
    pointer_size: PointerSize,
    endianness: Endianness,
) -> BlendFileParseResult<SimpleParsedBlock> {
    let (code, block_data) = get_bytes::<4>(block_data)?;
    let (size, block_data) = parse_i32(block_data, endianness)?;
    let (memory_address, block_data) = match pointer_size {
        PointerSize::Bits32 => {
            parse_u32(block_data, endianness).map(|res| lmap(res, Either::Left))?
        }
        PointerSize::Bits64 => {
            parse_u64(block_data, endianness).map(|res| lmap(res, Either::Right))?
        }
    };

    let (dna_index, block_data) = parse_u32(block_data, endianness)?;
    let (count, block_data) = parse_u32(block_data, endianness)?;
    // if size as usize != block_data.len() {
    //     panic!(
    //         "Something smells, {size}, {}, {}",
    //         block_data.len(),
    //         String::from_utf8(code.to_vec()).unwrap()
    //     )
    // }
    let (data, block_data) = get_byte_vec(block_data, size as usize)?;

    Ok((
        SimpleParsedBlock {
            code,
            size,
            memory_address,
            dna_index,
            count,
            data,
        },
        block_data,
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
    let (type_lengths, data) = multi(data, types_len, |d| parse_i16(d, endianness))?;

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
    print_i32(block.size, endianness, out);
    match block.memory_address {
        Either::Left(mem) => print_u32(mem, endianness, out),
        Either::Right(mem) => print_u64(mem, endianness, out),
    };
    print_u32(block.dna_index, endianness, out);
    print_u32(block.count, endianness, out);
    out.extend(block.data);
}

fn parse_blend_manual(blend_data: Vec<u8>) -> Result<ParsedBlendFile, BlendFileParseError> {
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

fn is_pointer(name: &str) -> bool {
    name.starts_with('*')
}

#[derive(Debug, Clone)]
enum FieldType {
    Value,
    ValueArray { dimensions: Vec<usize> },
    Pointer,
    FnPointer,
}

fn parse_field_type(name: &str, re: &Regex) -> FieldType {
    if name.starts_with('*') || name.starts_with("**") {
        return FieldType::Pointer;
    }
    if name.starts_with("(*") {
        return FieldType::FnPointer;
    }

    if name.contains('[') {
        let counts = re
            .captures_iter(name)
            .map(|c| c[1].parse::<usize>().unwrap())
            .collect();
        return FieldType::ValueArray { dimensions: counts };
    }

    FieldType::Value
}

fn count_from_dimensions(dims: &[usize]) -> usize {
    dims.iter().product()
}

pub type OffsetsWithPointerValue = Vec<(usize, Either<u32, u64>)>;

pub struct BlockContentWithPointers {
    pub simple_block: SimpleParsedBlock,
    pub original_mem_address: Either<u32, u64>,
    pub pointers: OffsetsWithPointerValue,
}

pub fn parse_block_contents(
    block: SimpleParsedBlock,
    pointer_size: PointerSize,
    endianness: Endianness,
    field_meta_lookup: &FieldMetaLookup,
) -> BlockContentWithPointers {
    let original_mem_address = block.memory_address;

    if &block.code == b"DNA1" {
        return BlockContentWithPointers {
            simple_block: block,
            original_mem_address,
            pointers: vec![],
        };
    }

    let mut pointers: OffsetsWithPointerValue = vec![];
    let fields = field_meta_lookup.get(&block.dna_index);
    if fields.is_none() {
        return BlockContentWithPointers {
            simple_block: block,
            original_mem_address,
            pointers,
        };
    }
    let fields = fields.unwrap();

    let ptr_size = match pointer_size {
        PointerSize::Bits32 => 4,
        PointerSize::Bits64 => 8,
    };

    for &offset in fields {
        let range_lo = offset;
        let range_hi = std::cmp::min(offset + ptr_size, block.size as usize);
        if offset + ptr_size >= block.size as usize {
            continue;
        }

        let data_for_field = &block.data[range_lo..range_hi].to_vec();
        match pointer_size {
            PointerSize::Bits32 => {
                if let Ok(data) = std::convert::TryInto::<[u8; 4]>::try_into(data_for_field.clone())
                {
                    let from_fn = match endianness {
                        Endianness::Little => u32::from_le_bytes,
                        Endianness::Big => u32::from_be_bytes,
                    };
                    pointers.push((offset, Either::Left(from_fn(data))));
                }
            }
            PointerSize::Bits64 => {
                if let Ok(data) = std::convert::TryInto::<[u8; 8]>::try_into(data_for_field.clone())
                {
                    let from_fn = match endianness {
                        Endianness::Little => u64::from_le_bytes,
                        Endianness::Big => u64::from_be_bytes,
                    };
                    pointers.push((offset, Either::Right(from_fn(data))))
                }
            }
        }
    }

    BlockContentWithPointers {
        simple_block: block,
        original_mem_address,
        pointers,
    }
}

fn scrub_block(
    mut block: SimpleParsedBlock,
    pointers: &Vec<(usize, Either<u32, u64>)>,
) -> SimpleParsedBlock {
    for (offset, pointer) in pointers {
        let data = match pointer {
            Either::Left(_) => vec![0, 0, 0, 0],
            Either::Right(_) => vec![0, 0, 0, 0, 0, 0, 0, 0],
        };

        block.data[*offset..*offset + data.len()].copy_from_slice(&data)
    }
    block.memory_address = match block.memory_address {
        Either::Left(_) => Either::Left(0),
        Either::Right(_) => Either::Right(0),
    };
    block
}

pub fn restore_block(
    mut block: SimpleParsedBlock,
    original_mem_address: Either<u32, u64>,
    pointers: &Vec<(usize, Either<u32, u64>)>,
) -> SimpleParsedBlock {
    block.memory_address = original_mem_address;
    for (offset, pointer) in pointers {
        let ptr_bytes = match pointer {
            Either::Left(p) => p.to_le_bytes().to_vec(),
            Either::Right(p) => p.to_le_bytes().to_vec(),
        };

        for (i, &byte) in ptr_bytes.iter().enumerate() {
            if let Some(elem) = block.data.get_mut(offset + i) {
                *elem = byte
            }
        }
    }

    block
}

pub struct BlendFileWithPointerData {
    pub header: Header,
    pub blocks: Vec<BlockContentWithPointers>,
}

pub fn parse_blend(blend_data: Vec<u8>) -> Result<BlendFileWithPointerData, BlendFileParseError> {
    let parsed_blend_file = parse_blend_manual(blend_data)?;
    let sdna = measure_time!("Finding SDNA block", {
        parsed_blend_file
            .blocks
            .iter()
            .find(|b| &b.code == b"DNA1")
            .expect("No DNA block found")
    });

    let (sdna_info, _) = measure_time!("Parsing SDNA", {
        parse_sdna(&sdna.data, parsed_blend_file.header.endianness)
            .expect("Cannot parse SDNA block")
    });

    let lookup = make_field_meta_lookup(&sdna_info, parsed_blend_file.header.pointer_size);

    let blocks_with_pointer_data = measure_time!("Scrubbing pointers", {
        parsed_blend_file
            .blocks
            .into_par_iter()
            .map(|block| {
                let block_content = parse_block_contents(
                    block,
                    parsed_blend_file.header.pointer_size,
                    parsed_blend_file.header.endianness,
                    &lookup,
                );
                let scrubbed_block =
                    scrub_block(block_content.simple_block, &block_content.pointers);
                BlockContentWithPointers {
                    simple_block: scrubbed_block,
                    original_mem_address: block_content.original_mem_address,
                    pointers: block_content.pointers,
                }
            })
            .collect()
    });

    Ok(BlendFileWithPointerData {
        header: parsed_blend_file.header,
        blocks: blocks_with_pointer_data,
    })
}

pub fn print_blend(mut blend_file: BlendFileWithPointerData, out: &mut Vec<u8>) {
    let restored_blocks: Vec<SimpleParsedBlock> = measure_time!("Restoring blocks", {
        blend_file
            .blocks
            .par_iter_mut()
            .map(|block| {
                restore_block(
                    block.simple_block.to_owned(),
                    block.original_mem_address,
                    &block.pointers,
                )
            })
            .collect()
    });

    let parsed_blend = ParsedBlendFile {
        header: blend_file.header,
        blocks: restored_blocks,
    };

    print_blend_manual(parsed_blend, out);
}

pub type FieldMetaLookup = HashMap<u32, Vec<usize>>;

pub fn make_field_meta_lookup(sdna_info: &DNAInfo, pointer_size: PointerSize) -> FieldMetaLookup {
    let mut result: HashMap<u32, Vec<usize>> = HashMap::new();
    let ptr_size = match pointer_size {
        PointerSize::Bits32 => 4,
        PointerSize::Bits64 => 8,
    };

    let re = Regex::new(r"\[(\d+)\]").unwrap();

    for (index, dna_struct) in sdna_info.structs.iter().enumerate() {
        let mut offset: usize = 0;
        let mut ptr_offsets: Vec<usize> = vec![];

        for field in &dna_struct.fields {
            let name = sdna_info.names[field.name_idx as usize].clone();
            let field_type = parse_field_type(&name, &re);
            let size_from_sdna = sdna_info.type_lengths[field.type_idx as usize];
            let size = match &field_type {
                FieldType::Value => size_from_sdna as usize,
                FieldType::ValueArray { dimensions } => {
                    count_from_dimensions(dimensions) * size_from_sdna as usize
                }
                FieldType::FnPointer => ptr_size,
                FieldType::Pointer { .. } => ptr_size,
            };

            if is_pointer(&name) {
                ptr_offsets.push(offset);
            }

            offset += size;
        }

        if !ptr_offsets.is_empty() {
            result.insert(index as u32, ptr_offsets);
        }
    }

    result
}
