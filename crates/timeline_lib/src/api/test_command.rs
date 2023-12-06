use crate::blend::{parse_print_blend::parse_blend, utils::from_file};

pub fn run_command_test(from_file_path: String) {
    let blend_bytes = from_file(&from_file_path).expect("cannot unpack blend file");

    let parsed_blend_file = parse_blend(blend_bytes).expect("cannot parse blend file");
}
