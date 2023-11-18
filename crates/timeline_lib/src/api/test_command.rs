use crate::{
    api::common::{parse_blend_manual, parse_sdna},
    blend::utils::from_file,
};

pub fn run_command_test(from_file_path: String) {
    let blend_bytes = from_file(&from_file_path).expect("cannot unpack blend file");

    let parsed_blend_file = parse_blend_manual(blend_bytes).expect("cannot parse blend file");

    let sdna = parsed_blend_file
        .blocks
        .into_iter()
        .find(|b| &b.code == b"DNA1")
        .expect("whoops");

    let (sdna_info, _) =
        parse_sdna(&sdna.data, parsed_blend_file.header.endianness).expect("whoops, sdna");

    println!("{:?}", sdna_info.names)
}
