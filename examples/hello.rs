extern crate timeline;
use timeline::api::{create_new_checkpoint_command, init_command};

fn main() {
    // init_command::init_db(
    //     "data/blender-3.3-splash.blend.db",
    //     "22",
    //     "data/blender-3.3-splash.blend",
    // )
    // .unwrap();

    create_new_checkpoint_command::create_new_checkpoint(
        "data/blender-3.3-splash.blend",
        "data/blender-3.3-splash.blend.db",
        None,
    )
    .unwrap();
}
