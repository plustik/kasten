mod dir;
mod user;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: bonbon <COMMAND> [<ARGUMENTS>]");
        return;
    }

    match args[1].as_str() {
        "useradd" => {
            user::useradd(args);
        }
        "userlist" => {
            user::userlist(args);
        }
        "userrm" => {
            user::userrm(args);
        }
        "dirlist" => {
            dir::dirlist(args);
        }
        "diradd" => {
            dir::diradd(args);
        }
        "help" | "-h" | "--help" => {
            println!("Usage: bonbon <COMMAND> [<ARGUMENTS>]");
            println!("COMMANDS:");
            println!("'useradd <db-location> <username> <password>'");
            println!("'userlist <db-location>'");
            println!("'userrm <db-location> <user-id>'");
            println!("'dirlist <db-location>'");
            println!("'diradd <db-location> <dirname> <parent_id> <owner_id>'");
        }
        _ => {
            println!("COMMANDS:\n'useradd'\n'userlist'\n'userrm'\n'help'");
        }
    }
}
