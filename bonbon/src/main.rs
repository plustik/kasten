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
        _ => {
            println!("COMMANDS:\n'useradd'\n'userlist'");
        }
    }
}
