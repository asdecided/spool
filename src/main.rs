use asdecided_spool::{Journal, Result};
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("spool: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args == ["--help"] || args == ["help"] {
        println!(
            "Spool — pick up where you left off.\nFoundation prototype: records jobs and JSON checkpoints only.\n\nspool init\nspool new <goal>\nspool status\nspool inspect <job-id>\nspool checkpoint <job-id> <json>"
        );
        return Ok(());
    }
    let path = Path::new(".spool/journal.sqlite3");
    if args == ["init"] {
        std::fs::create_dir_all(".spool")?;
        Journal::open(path)?;
        println!("Initialised .spool/journal.sqlite3");
        return Ok(());
    }
    if !path.is_file() {
        return Err("run `spool init` first".into());
    }
    let journal = Journal::open(path)?;
    match args[0].as_str() {
        "new" if args.len() == 2 => println!("{}", journal.create(&args[1])?),
        "status" if args.len() == 1 => {
            println!("{}", serde_json::to_string_pretty(&journal.list()?)?)
        }
        "inspect" if args.len() == 2 => println!(
            "{}",
            serde_json::to_string_pretty(&journal.inspect(args[1].parse()?)?)?
        ),
        "checkpoint" if args.len() == 3 => {
            println!("{}", journal.checkpoint(args[1].parse()?, &args[2])?)
        }
        _ => return Err("invalid arguments; see `spool --help`".into()),
    }
    Ok(())
}
