mod cli;

use cli::{parse_args, Commands};

use log::error;
use timeline_lib::{
    api::{
        commit_command::create_new_commit,
        delete_branch::delete_branch,
        export_descendants_of_commit::export_descendants_of_commit,
        get_current_branch::get_current_branch,
        import_exchange,
        init_command::init_db,
        init_from_import_command,
        list_branches_command::list_braches,
        log_checkpoints_command::list_checkpoints,
        new_branch_command::create_new_branch,
        prepare_sync::prepare_sync,
        restore_command::restore_checkpoint,
        switch_command::switch_branches,
        test_command::run_command_test,
        utils::{read_exchange_from_file, write_exchange_to_file},
    },
    db::db_ops::DBError,
    exchange::structs::{decode_exchange, encode_sync},
};

fn print_error_discard_rest<T>(res: Result<T, DBError>) {
    match res {
        Ok(_) => {}
        Err(err) => error!("{}", err),
    }
}

fn print_all_branches(db_path: &str) {
    let result = list_braches(db_path);
    match result {
        Ok(branches) => branches
            .into_iter()
            .for_each(|branch| println!("{}", branch)),
        Err(err) => error!("{}", err),
    }
}

fn print_checkpoints(db_path: &str, branch_name: &str) {
    let result = list_checkpoints(db_path, branch_name);
    match result {
        Ok(checkpoints) => checkpoints
            .into_iter()
            .for_each(|commit| println!("{} {} {}", commit.hash, commit.branch, commit.message)),
        Err(err) => error!("{}", err),
    }
}

fn run_new_branch_command(db_path: &str, new_branch_name: &str) {
    print_error_discard_rest(create_new_branch(db_path, new_branch_name));
}

fn run_create_new_commit(db_path: &str, file_path: &str, message: Option<String>) {
    print_error_discard_rest(create_new_commit(file_path, db_path, message));
}

fn run_restore_checkpoint(db_path: &str, file_path: &str, hash: &str) {
    print_error_discard_rest(restore_checkpoint(file_path, db_path, hash));
}

fn run_switch_branches(db_path: &str, file_path: &str, branch_name: &str) {
    print_error_discard_rest(switch_branches(db_path, branch_name, file_path));
}

fn run_get_current_branch(db_path: &str) {
    let result = get_current_branch(db_path);
    match result {
        Ok(branch) => println!("Current branch: {}", branch),
        Err(err) => error!("{}", err),
    }
}

fn run_init_command(db_path: &str, file_path: &str) {
    let project_id = uuid::Uuid::new_v4().to_string();
    print_error_discard_rest(init_db(db_path, &project_id, file_path));
}

fn run_export_command(db_path: &str, root_hash: &str, path_to_file: &str) {
    let exchange = export_descendants_of_commit(db_path, root_hash).expect("Cannot export");
    write_exchange_to_file(&exchange, path_to_file).expect("Cannot write exchange file");
}

fn run_import_command(db_path: &str, path_to_exchange: &str) {
    let exchange =
        read_exchange_from_file(path_to_exchange).expect("Cannot read exchange from file");
    import_exchange::import_exchange(db_path, exchange).expect("Cannot import exchange");
}

fn run_delete_branch_command(db_path: &str, branch_name: &str) {
    delete_branch(db_path, branch_name).expect("Cannot delete branch")
}

#[inline]
fn v1_sync_url(base: &str) -> String {
    format!("{}/v1/sync", base)
}

fn sync_to_server(db_path: &str, url: &str) {
    let sync = prepare_sync(db_path).expect("Cannot prepare sync");
    let sync_data = encode_sync(&sync).expect("Cannot encode sync");
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(v1_sync_url(url))
        .body(sync_data)
        .send()
        .expect("Cannot send sync request");

    if !res.status().is_success() {
        println!("Error: {}", res.status().as_str());
        return;
    }

    let exchange_data = res.bytes().expect("Cannot read response body");
    let exchange = decode_exchange(&exchange_data).expect("Cannot decode exchange");
    import_exchange::import_exchange(db_path, exchange).expect("Cannot import exchange file");
}

fn run_init_from_import(url: &str, db_path: &str, file_path: &str, project_id: &str) {
    let res = reqwest::blocking::get(format!("{}/v1/clone/{}", url, project_id))
        .expect("Cannot send import request");
    if !res.status().is_success() {
        println!("Error: {}", res.status().as_str());
        return;
    }

    let exchange_data = res.bytes().expect("Cannot read response body");
    let exchange = decode_exchange(&exchange_data).expect("Cannot decode exchange");

    init_from_import_command::init_from_import_command(db_path, exchange, file_path)
        .expect("Cannot init db from import");
}

fn main() {
    let args = parse_args();
    env_logger::init();
    match args.command {
        Commands::Test { from_path, to_path } => run_command_test(from_path, to_path),
        Commands::Commit {
            db_path,
            file_path,
            message,
        } => run_create_new_commit(&db_path, &file_path, message),
        Commands::Restore {
            db_path,
            file_path,
            hash,
        } => run_restore_checkpoint(&db_path, &file_path, &hash),
        Commands::NewBranch {
            db_path,
            branch_name,
        } => run_new_branch_command(&db_path, &branch_name),
        Commands::ListBranches { db_path } => print_all_branches(&db_path),
        Commands::GetCurrentBranch { db_path } => run_get_current_branch(&db_path),
        Commands::Switch {
            db_path,
            branch,
            file_path,
        } => run_switch_branches(&db_path, &file_path, &branch),
        Commands::LogCheckpoints { db_path, branch } => print_checkpoints(&db_path, &branch),
        Commands::Init { db_path, file_path } => run_init_command(&db_path, &file_path),
        Commands::Export {
            db_path,
            from_commit,
            path_to_exchange,
        } => run_export_command(&db_path, &from_commit, &path_to_exchange),
        Commands::Import {
            db_path,
            path_to_exchange,
        } => run_import_command(&db_path, &path_to_exchange),
        Commands::DeleteBranch {
            db_path,
            branch_name,
        } => run_delete_branch_command(&db_path, &branch_name),
        Commands::SyncToServer { db_path, url } => sync_to_server(&db_path, &url),
        Commands::InitFromImport {
            url,
            db_path,
            file_path,
            project_id,
        } => run_init_from_import(&url, &db_path, &file_path, &project_id),
    }
}
