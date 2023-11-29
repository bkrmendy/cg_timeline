use std::{io::Write, time::Instant};

use flate2::write::GzDecoder;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    api::common::parse_hash_list,
    blend::utils::to_file_transactional,
    db::db_ops::{DBError, Persistence, DB},
    measure_time,
};

pub fn restore_checkpoint(file_path: &str, db_path: &str, hash: &str) -> Result<(), DBError> {
    let end_to_end_timer = Instant::now();

    let mut conn = Persistence::open(db_path)?;

    let commit = measure_time!(format!("Reading commit {:?}", hash), {
        conn.read_commit(hash)?
            .ok_or(DBError::Consistency("no such commit found".to_owned()))
    })?;

    let block_hashes = measure_time!(format!("Reading blocks {:?}", hash), {
        parse_hash_list(commit.blocks)
    });

    let header = commit.header;

    let block_data: Vec<Vec<u8>> = measure_time!(format!("Decompressing blocks {:?}", hash), {
        conn.read_blocks(block_hashes)
            .map_err(|_| DBError::Error("Cannot read block hashes".to_owned()))?
            .par_iter()
            .map(|record| {
                let mut writer = Vec::new();
                let mut deflater = GzDecoder::new(writer);
                deflater.write_all(&record.data).unwrap();
                writer = deflater.finish().unwrap();
                writer
            })
            .collect()
    });

    measure_time!(format!("Writing file {:?}", hash), {
        to_file_transactional(file_path, header, block_data, b"ENDB".to_vec())
            .map_err(|_| DBError::Fundamental("Cannot write to file".to_owned()))?;
    });

    conn.execute_in_transaction(|tx| {
        Persistence::write_current_branch_name(tx, &commit.branch)?;
        Persistence::write_current_commit_pointer(tx, &commit.hash)?;
        Ok(())
    })?;

    println!("Checkout took: {:?}", end_to_end_timer.elapsed());

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
