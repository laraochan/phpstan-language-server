mod phpstan;
mod protocol;
mod server;

use std::io::{self, BufReader};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    server::Server::new()?.run(BufReader::new(stdin.lock()), stdout.lock())
}
