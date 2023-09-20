use crate::{
    api::common::{parse_blend_manual, print_blend_manual},
    blend::utils::{from_file, to_file_transactional},
};

pub fn run_command_test(from_file_path: String, to_file_path: String) {
    let blend_bytes = from_file(&from_file_path).expect("cannot unpack blend file");

    let parsed_blend_file = parse_blend_manual(blend_bytes).expect("cannot parse blend file");
    println!(
        "{:?} - {:?}",
        parsed_blend_file.header,
        parsed_blend_file.blocks.len()
    );

    let mut write_back: Vec<u8> = vec![];
    print_blend_manual(parsed_blend_file, &mut write_back);

    let p1 = vec![];
    let p2 = vec![];

    to_file_transactional(&to_file_path, write_back, p1, p2).expect("cannot write to file")
}
