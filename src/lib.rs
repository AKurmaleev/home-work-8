//! Homework 8: a safe fixed-capacity container and a Hangman state machine.
//!
//! See [`MyVec`] for the independent container exercise. The game separates
//! validation from checking; the console consumes the game and borrows streams.
//!
//! ```
//! use std::io::Cursor;
//! use rust_homework_8::{console, game::{Game, GameState, GuessResult}};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut game = Game::new("rust", 5)?;
//! game.submit_guess("R")?;
//! assert_eq!(game.state(), GameState::CheckingGuess('r'));
//! assert_eq!(game.masked_word(), "_ _ _ _");
//! assert_eq!(game.check_guess(), Some(GuessResult::Hit));
//! assert_eq!(game.state(), GameState::WaitingForInput);
//!
//! let mut input = Cursor::new(b"u\ns\nt\n");
//! let mut output = Vec::new();
//! console::run(&mut input, &mut output, game)?;
//! assert!(String::from_utf8(output)?.ends_with("Вы выиграли! Правильное слово: rust\n"));
//! # Ok(())
//! # }
//! ```

pub mod console;
pub mod game;
pub mod my_vec;

pub use my_vec::{MyVec, MyVecIter};
