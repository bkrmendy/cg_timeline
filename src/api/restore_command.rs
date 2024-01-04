use std::{io::Write, iter::zip, time::Instant};

use flate2::write::GzDecoder;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    api::common::parse_blocks_and_pointers,
    blend::{
        parse_print_blend::{
            parse_block_manual, parse_header_manual, print_blend, BlendFileWithPointerData,
            BlockContentWithPointers,
        },
        utils::to_file_transactional,
    },
    db::db_ops::{DBError, Persistence, DB},
    measure_time,
};

pub fn restore_checkpoint(file_path: &str, db_path: &str, hash: &str) -> Result<(), DBError> {
    let restore_command_timer = Instant::now();

    let mut conn = Persistence::open(db_path)?;

    let commit = measure_time!(format!("Reading commit {:?}", hash), {
        conn.read_commit(hash)?
            .ok_or(DBError::Consistency("no such commit found".to_owned()))
    })?;

    let block_meta = measure_time!(format!("Reading blocks {:?}", hash), {
        parse_blocks_and_pointers(&commit.blocks_and_pointers)
    });

    let header_data = commit.header;
    let (header, _) = parse_header_manual(&header_data).unwrap();

    let blocks: Vec<BlockContentWithPointers> =
        measure_time!(format!("Decompressing blocks {:?}", hash), {
            let block_hashes = block_meta.iter().map(|b| b.hash.clone()).collect();

            let blocks_minus_pointers: Vec<Vec<u8>> = conn
                .read_blocks(block_hashes)
                .map_err(|_| DBError::Error("Cannot read block hashes".to_owned()))?
                .par_iter()
                .map(|record| {
                    let mut writer = Vec::new();
                    let mut deflater = GzDecoder::new(writer);
                    deflater.write_all(&record.data).unwrap();
                    writer = deflater.finish().unwrap();
                    writer
                })
                .collect();

            zip(block_meta, blocks_minus_pointers)
                .map(|(meta, data)| {
                    let (simple_block, _) =
                        parse_block_manual(&data, header.pointer_size, header.endianness).unwrap();

                    BlockContentWithPointers {
                        simple_block,
                        original_mem_address: meta.original_mem_address,
                        pointers: meta.pointers,
                    }
                })
                .collect()
        });

    let mut out: Vec<u8> = vec![];
    print_blend(BlendFileWithPointerData { header, blocks }, &mut out);

    measure_time!(format!("Writing file {:?}", hash), {
        to_file_transactional(file_path, out, b"ENDB".to_vec())
            .map_err(|_| DBError::Fundamental("Cannot write to file".to_owned()))?;
    });

    conn.execute_in_transaction(|tx| {
        Persistence::write_current_branch_name(tx, &commit.branch)?;
        Persistence::write_current_commit_pointer(tx, &commit.hash)?;
        Ok(())
    })?;

    println!("Checkout took: {:?}", restore_command_timer.elapsed());

    Ok(())
}

#[cfg(test)]
mod test {
    use tempfile::NamedTempFile;

    use crate::{
        api::{init_command::MAIN_BRANCH_NAME, test_utils},
        db::db_ops::{Persistence, DB},
    };

    use super::restore_checkpoint;

    #[test]
    fn test_restore() {
        let tmp_file = NamedTempFile::new().expect("Cannot create temp dir");
        let tmp_path = tmp_file.path().to_str().expect("Cannot get temp file path");

        test_utils::init_db_from_file(tmp_path, "my-cool-project", "data/fixtures/untitled.blend");

        test_utils::commit(tmp_path, "Commit", "data/fixtures/untitled_2.blend");
        test_utils::commit(tmp_path, "Commit 2", "data/fixtures/untitled_3.blend");

        let tmp_blend_path = NamedTempFile::new().expect("Cannot create temp file");

        restore_checkpoint(
            tmp_blend_path.path().to_str().unwrap(),
            tmp_path,
            "b637ec695e10bed0ce06279d1dc46717",
        )
        .expect("Cannot restore checkpoint");

        // Number of commits stays the same
        assert_eq!(
            test_utils::list_checkpoints(tmp_path, MAIN_BRANCH_NAME).len(),
            3
        );

        let db = Persistence::open(tmp_path).expect("Cannot open test DB");

        let current_branch_name = db
            .read_current_branch_name()
            .expect("Cannot read current branch name");

        // The current branch name stays the same
        assert_eq!(current_branch_name, MAIN_BRANCH_NAME);

        let latest_commit_hash = db
            .read_current_commit_pointer()
            .expect("Cannot read latest commit");

        // The latest commit hash is updated to the hash of the restored commit
        assert_eq!(latest_commit_hash, "b637ec695e10bed0ce06279d1dc46717");

        // The tip of `main` stays the same
        let main_tip = db.read_branch_tip(MAIN_BRANCH_NAME).unwrap().unwrap();
        assert_eq!(main_tip, "d9e8eb09f8270ad5326de946d951433a");
    }
}
