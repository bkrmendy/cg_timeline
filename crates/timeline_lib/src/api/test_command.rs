use std::collections::HashMap;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use regex::Regex;

use crate::{
    api::common::{parse_blend_manual, parse_sdna},
    blend::{
        blend_file::{BlockWithParsedFields, DNAInfo, DNAStruct, ParsedField, SimpleParsedBlock},
        utils::{from_file, Either},
    },
};

fn is_pointer(name: &str) -> bool {
    name.starts_with('*')
}

fn has_pointer(sdna_info: &DNAInfo, dna_struct: &DNAStruct) -> bool {
    dna_struct.fields.iter().any(|field| {
        let name = &sdna_info.names[field.name_idx as usize];
        is_pointer(name)
    })
}

fn get_mam_addr(addr: Either<u32, u64>) -> u64 {
    match addr {
        Either::Left(a) => a as u64,
        Either::Right(a) => a,
    }
}

#[derive(Debug, Clone)]
enum FieldType {
    Value,
    ValueArray { dimensions: Vec<usize> },
    Pointer { indirection_count: usize },
    FnPointer,
}

fn parse_field_type(name: &str) -> FieldType {
    if name.starts_with("**") {
        return FieldType::Pointer {
            indirection_count: 2,
        };
    }
    if name.starts_with('*') {
        return FieldType::Pointer {
            indirection_count: 1,
        };
    }
    if name.starts_with("(*") {
        return FieldType::FnPointer;
    }

    let re = Regex::new(r"\[(\d+)\]").unwrap();

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

fn parse_block_contents(block: SimpleParsedBlock, sdna_info: &DNAInfo) -> BlockWithParsedFields {
    let mut offset: usize = 0;
    let mut fields: Vec<ParsedField> = vec![];
    let fields_meta = &sdna_info.structs[block.dna_index as usize].fields;
    for field in fields_meta {
        let name = sdna_info.names[field.name_idx as usize].clone();
        let type_name = &sdna_info.types[field.type_idx as usize];
        let field_type = parse_field_type(&name);
        let size_from_sdna = sdna_info.type_lengths[field.type_idx as usize];
        let size = match &field_type {
            FieldType::Value => size_from_sdna as usize,
            FieldType::ValueArray { dimensions } => {
                count_from_dimensions(dimensions) * size_from_sdna as usize
            }
            FieldType::FnPointer => 8_usize, // TODO: pointer size
            FieldType::Pointer { .. } if type_name == "DrawData" => {
                let size = block.size as usize - offset;
                println!(
                    "{:?} {size} {} {name} {:?}",
                    String::from_utf8(block.code.to_vec()).unwrap(),
                    block.dna_index,
                    field_type
                );
                size
            }
            FieldType::Pointer { .. } => 8_usize,
        };

        let data_for_field = if size > 0 {
            block.data[offset..offset + size].to_vec()
        } else {
            vec![]
        };

        let points_to = if size == 8 && is_pointer(&name) {
            let data: [u8; 8] =
                std::convert::TryInto::<[u8; 8]>::try_into(data_for_field.clone()).unwrap();
            Some(u64::from_le_bytes(data))
        } else {
            None
        };

        offset += size;
        fields.push(ParsedField {
            name,
            points_to,
            data_for_field,
        })
    }

    BlockWithParsedFields {
        code: block.code,
        size: block.size,
        memory_address: block.memory_address,
        dna_index: block.dna_index,
        count: block.count,
        data: fields.clone(),
    }
}

pub fn run_command_test(from_file_path: String) {
    let blend_bytes = from_file(&from_file_path).expect("cannot unpack blend file");

    let parsed_blend_file = parse_blend_manual(blend_bytes).expect("cannot parse blend file");

    let sdna = parsed_blend_file
        .blocks
        .iter()
        .find(|b| &b.code == b"DNA1")
        .expect("whoops");

    let (sdna_info, _) =
        parse_sdna(&sdna.data, parsed_blend_file.header.endianness).expect("whoops, sdna");

    let addr_map: HashMap<u64, SimpleParsedBlock> = HashMap::from_iter(
        parsed_blend_file
            .blocks
            .iter()
            .map(|block| (get_mam_addr(block.memory_address), block.clone())),
    );

    let blocks_with_parsed_contents: Vec<BlockWithParsedFields> = parsed_blend_file
        .blocks
        .into_par_iter()
        .map(|block| parse_block_contents(block, &sdna_info))
        .collect();
}
