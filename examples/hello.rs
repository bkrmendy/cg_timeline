extern crate timeline;
use timeline::api::init_command;

fn main() {
    init_command::init_db(
        "data/blender-splash.blend.db",
        "22",
        "data/blender-splash.blend",
    )
    .unwrap();
}
