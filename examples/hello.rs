extern crate timeline;
use anyhow::Context;
use timeline::api::delete_branch;

fn main() -> anyhow::Result<()> {
    delete_branch::delete_branch("data/untitled_3.blend.timeline", "hello")
        .context("Cannot delete branch")?;
    Ok(())
}
