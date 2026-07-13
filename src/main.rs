mod cli;
mod phpstan;
mod protocol;
mod server;

use cli::CommandLineAction;
use std::{
    env,
    io::{self, BufReader},
    process,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(2);
    }
}

fn run() -> io::Result<()> {
    match cli::parse(env::args().skip(1)).map_err(io::Error::other)? {
        CommandLineAction::Help => {
            print!("{}", cli::HELP);
            Ok(())
        }
        CommandLineAction::Start(options) => {
            let stdin = io::stdin();
            let stdout = io::stdout();
            server::Server::new(options)?.run(BufReader::new(stdin.lock()), stdout.lock())
        }
    }
}
