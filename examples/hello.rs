extern crate timeline;
use timeline::api::blend_file_from_timeline_command;

fn main() {
    blend_file_from_timeline_command::blend_file_from_timeline("data/untitled.blend.timeline")
        .unwrap();
}
