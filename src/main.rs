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
            "Spool — pick up where you left off.\nLocal Linux execution preview v{}\n\nspool init\nspool doctor\nspool new <goal>\nspool status\nspool inspect <job-id>\nspool checkpoint <job-id> <json>\nspool queue <job-id> -- <command> [args...]\nspool exec <job-id> -- <command> [args...]\nspool resume <job-id>\nspool recover\nspool resolve <operation-id> succeeded|failed <evidence>\nspool events <operation-id>\n\nCommands run offline inside a bubblewrap sandbox with a per-job /work directory.\nTimeout: 60 seconds; override with SPOOL_TIMEOUT_SECONDS (1..3600).",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    if args == ["--version"] {
        println!("spool {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args == ["doctor"] {
        println!("{}", asdecided_spool::execution::doctor()?);
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
        "queue" | "exec" if args.len() >= 4 && args[2] == "--" => {
            let job = args[1].parse()?;
            let timeout = timeout()?;
            let operation = journal.queue(job, &args[3..])?;
            println!("Queued operation {operation}");
            if args[0] == "exec" {
                journal.resume(job, Path::new(".spool"), timeout)?;
                println!("{}", journal.inspect(job)?);
            }
        }
        "resume" if args.len() == 2 => {
            let job = args[1].parse()?;
            journal.resume(job, Path::new(".spool"), timeout()?)?;
            println!("{}", journal.inspect(job)?);
        }
        "recover" if args.len() == 1 => println!(
            "Marked {} interrupted operations uncertain; nothing replayed",
            journal.recover()?
        ),
        "resolve" if args.len() == 4 => journal.resolve(args[1].parse()?, &args[2], &args[3])?,
        "events" if args.len() == 2 => println!(
            "{}",
            serde_json::to_string_pretty(&journal.events(args[1].parse()?)?)?
        ),
        _ => return Err("invalid arguments; see `spool --help`".into()),
    }
    Ok(())
}

fn timeout() -> Result<std::time::Duration> {
    let seconds: u64 = std::env::var("SPOOL_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| "60".into())
        .parse()?;
    if !(1..=3600).contains(&seconds) {
        return Err("timeout must be 1..3600 seconds".into());
    }
    Ok(std::time::Duration::from_secs(seconds))
}
