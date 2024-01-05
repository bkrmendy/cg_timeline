use anyhow::bail;

use crate::db::db_ops::{DBError, Persistence, DB};
use std::path::Path;

use super::restore_command::restore_checkpoint;

pub fn blend_file_from_timeline(db_path: &str) -> anyhow::Result<String> {
    let conn = Persistence::open(db_path)?;
    let tip = conn.read_branch_tip("main")?;
    if tip.is_none() {
        bail!(DBError::Consistency(String::from(
            "timeline has no main branch",
        )))
    }
    let tip = tip.unwrap();

    let path = Path::new(db_path);
    // TODO: should check if the filename ends in .blend
    let dir = path.parent().unwrap();
    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("restored.blend");

    let blend_file_path_buf = dir.join(file_name);
    let blend_file_path = blend_file_path_buf.to_str().unwrap();
    println!("{blend_file_path}");

    restore_checkpoint(blend_file_path, db_path, &tip)?;

    Ok(blend_file_path.to_string())
}
