//! Pure Hangman rules without console I/O.
//!
//! Words and guesses use Unicode scalar values, not bytes or grapheme clusters.
//! Case matching uses each scalar's locale-independent `char::to_lowercase`,
//! accepting only mappings to exactly one scalar. No Unicode normalization or
//! full case folding is performed.

use std::error::Error;
use std::fmt;

const MAX_WORD_CHARS: usize = 64;

/// Current phase of the game; a pending guess is validated but not yet applied.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GameState {
    /// At least one letter is hidden, attempts remain, and input is accepted.
    WaitingForInput,
    /// A normalized letter awaits checking; history and attempts are unchanged.
    CheckingGuess(char),
    /// Every position in the word has been revealed.
    Won,
    /// No attempts remain.
    Lost,
}

/// Outcome of checking a previously submitted guess.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GuessResult {
    /// A new letter revealed all its positions without consuming an attempt.
    Hit,
    /// A new absent letter consumed one attempt.
    Miss,
    /// The letter was already guessed; only the phase returns to waiting.
    Repeated,
}

/// Recoverable validation errors; invalid input never consumes an attempt.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GameError {
    /// The word is empty, exceeds 64 scalars, or contains an unsupported scalar.
    InvalidWord,
    /// The initial number of attempts is zero.
    InvalidAttempts,
    /// The trimmed guess is not one supported alphabetic scalar.
    InvalidGuess,
    /// A guess is already pending, or the game has ended.
    NotWaitingForInput,
}

impl fmt::Display for GameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWord => formatter.write_str(
                "word must contain 1 to 64 alphabetic Unicode scalars with single-scalar lowercase mappings",
            ),
            Self::InvalidAttempts => formatter.write_str("attempts must be greater than zero"),
            Self::NotWaitingForInput => formatter.write_str("game is not waiting for input"),
            Self::InvalidGuess => formatter.write_str(
                "guess must contain exactly one alphabetic Unicode scalar with a single-scalar lowercase mapping",
            ),
        }
    }
}

impl Error for GameError {}

/// An owned, validated word and its chronological, unique normalized guesses.
///
/// The game owns a nonempty word of at most 64 normalized alphabetic scalars.
/// History contains unique normalized guesses in checking order. Only checking
/// a new miss consumes an attempt. Waiting and checking require positive
/// attempts and a hidden position; a pending letter is normalized but unapplied.
/// Won means every position is revealed; lost means zero attempts remain.
/// Terminal states are immutable. Private fields preserve these invariants.
/// `Debug` and [`Game::secret_word`] expose the secret; console code should use
/// [`Game::masked_word`] during play and reveal the secret at the end.
#[derive(Debug)]
pub struct Game {
    word: Vec<char>,
    guessed_letters: Vec<char>,
    remaining_attempts: u8,
    state: GameState,
}

impl Game {
    /// Borrows `word` for validation and owns its normalized scalar values.
    ///
    /// Accepts 1 to 64 alphabetic Unicode scalars, each mapping to exactly one
    /// scalar through `char::to_lowercase`, and 1 to 255 attempts. The word is
    /// not trimmed. Validation stops at the first invalid scalar or the 65th
    /// scalar without allocating storage proportional to oversized input.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::InvalidAttempts`] when `attempts` is zero, or
    /// [`GameError::InvalidWord`] for an empty, oversized, or unsupported word.
    /// Invalid input is reported without panicking.
    pub fn new(word: &str, attempts: u8) -> Result<Self, GameError> {
        if attempts == 0 {
            return Err(GameError::InvalidAttempts);
        }

        let mut normalized_word = Vec::new();
        for letter in word.chars() {
            if normalized_word.len() == MAX_WORD_CHARS {
                return Err(GameError::InvalidWord);
            }
            let normalized = normalize_letter(letter).ok_or(GameError::InvalidWord)?;
            normalized_word.push(normalized);
        }

        if normalized_word.is_empty() {
            return Err(GameError::InvalidWord);
        }

        Ok(Self {
            word: normalized_word,
            guessed_letters: Vec::new(),
            remaining_attempts: attempts,
            state: GameState::WaitingForInput,
        })
    }

    /// Validates borrowed input and transitions from waiting to checking.
    ///
    /// Trims Unicode whitespace and accepts exactly one alphabetic scalar whose
    /// `char::to_lowercase` mapping contains one scalar. Only that normalized
    /// `char` is retained. This does not reveal positions, record history, or
    /// consume attempts, even for winning, losing, or repeated guesses.
    /// Call [`Game::check_guess`] separately to apply the pending letter.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::NotWaitingForInput`] before validating input if a
    /// guess is pending or the game is terminal. While waiting, returns
    /// [`GameError::InvalidGuess`] for empty, multi-scalar, nonalphabetic, or
    /// expanding-lowercase input. Every error leaves all fields unchanged.
    pub fn submit_guess(&mut self, input: &str) -> Result<(), GameError> {
        match self.state {
            GameState::WaitingForInput => {}
            GameState::CheckingGuess(_) | GameState::Won | GameState::Lost => {
                return Err(GameError::NotWaitingForInput);
            }
        }

        let mut letters = input.trim().chars();
        let letter = letters.next().ok_or(GameError::InvalidGuess)?;
        if letters.next().is_some() {
            return Err(GameError::InvalidGuess);
        }
        let normalized = normalize_letter(letter).ok_or(GameError::InvalidGuess)?;
        self.state = GameState::CheckingGuess(normalized);
        Ok(())
    }

    /// Applies the pending letter once and transitions to waiting, won, or lost.
    ///
    /// Hits reveal all matching positions without consuming attempts. New misses
    /// consume exactly one attempt. Repeats preserve history, mask, and attempts.
    /// Returns `None` without mutation when no guess is pending, including in
    /// terminal states. This operation performs no I/O and cannot reject input:
    /// the pending letter has already been validated by [`Game::submit_guess`].
    pub fn check_guess(&mut self) -> Option<GuessResult> {
        let letter = match self.state {
            GameState::CheckingGuess(letter) => letter,
            GameState::WaitingForInput | GameState::Won | GameState::Lost => return None,
        };

        let result = if self.guessed_letters.contains(&letter) {
            GuessResult::Repeated
        } else {
            self.guessed_letters.push(letter);
            if self.word.contains(&letter) {
                GuessResult::Hit
            } else {
                // CheckingGuess guarantees at least one remaining attempt.
                self.remaining_attempts -= 1;
                GuessResult::Miss
            }
        };

        self.state = if self.remaining_attempts == 0 {
            GameState::Lost
        } else if self
            .word
            .iter()
            .all(|letter| self.guessed_letters.contains(letter))
        {
            GameState::Won
        } else {
            GameState::WaitingForInput
        };
        Some(result)
    }

    /// Returns the stored phase by value without applying a pending guess.
    pub fn state(&self) -> GameState {
        self.state
    }

    /// Returns attempts remaining; only new misses decrease this value.
    pub fn remaining_attempts(&self) -> u8 {
        self.remaining_attempts
    }

    /// Borrows normalized, unique guesses in the order first checked.
    ///
    /// Includes checked hits and misses, but not an unapplied pending guess.
    /// The slice borrows this game, preventing
    /// mutation while that borrow remains in use.
    pub fn guessed_letters(&self) -> &[char] {
        &self.guessed_letters
    }

    /// Returns an owned display string with one ASCII space between positions.
    ///
    /// Hidden scalars are `_`; revealed scalars use the normalized lowercase
    /// word. There is no leading or trailing space, including after a loss.
    pub fn masked_word(&self) -> String {
        let mut masked = String::new();
        for (index, &letter) in self.word.iter().enumerate() {
            if index != 0 {
                masked.push(' ');
            }
            masked.push(if self.guessed_letters.contains(&letter) {
                letter
            } else {
                '_'
            });
        }
        masked
    }

    /// Returns an owned copy of the normalized word without separating spaces.
    ///
    /// Available in every state for final output; calling it does not reveal
    /// positions in the game or otherwise mutate state.
    pub fn secret_word(&self) -> String {
        self.word.iter().copied().collect()
    }
}

fn normalize_letter(letter: char) -> Option<char> {
    if !letter.is_alphabetic() {
        return None;
    }

    let mut lowercase = letter.to_lowercase();
    let normalized = lowercase.next()?;
    if lowercase.next().is_some() {
        None
    } else {
        Some(normalized)
    }
}
