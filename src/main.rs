use std::io;

use rust_homework_8::{console, game::Game};

const SECRET_WORD: &str = "rust";
const ATTEMPTS: u8 = 5;

fn main() -> io::Result<()> {
    let game = Game::new(SECRET_WORD, ATTEMPTS).map_err(io::Error::other)?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    console::run(&mut stdin.lock(), &mut stdout.lock(), game)
}
