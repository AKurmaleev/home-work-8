//! Interactive state-machine driver and bounded UTF-8 line input.

use std::io::{self, BufRead, Read, Write};

use crate::game::{Game, GameState, GuessResult};

const MAX_LINE_BYTES: usize = 128;

/// Runs one owned game using borrowed input and output streams.
///
/// Waiting reads and validates input; checking applies the pending guess before
/// any further read. Terminal games print their final state without reading.
/// A game supplied in the checking phase processes its pending letter first.
/// Each line may contain at most 128 bytes, including its line terminator.
/// Oversized lines are discarded through newline or EOF without storing the
/// remainder. Invalid UTF-8 and invalid guesses do not change the game.
/// EOF ends the session without declaring a win or loss and reveals the word.
/// This interactive, blocking API has no input deadline.
///
/// # Errors
///
/// Propagates stream read, write, and flush errors. Prompts are flushed before
/// reading input, and final output is flushed before returning success.
pub fn run<R: BufRead, W: Write>(input: &mut R, output: &mut W, mut game: Game) -> io::Result<()> {
    writeln!(output, "Игра «Виселица». Вводите по одной букве.")?;
    let mut line = Vec::with_capacity(MAX_LINE_BYTES + 1);

    loop {
        match game.state() {
            GameState::WaitingForInput => {
                show_state(output, &game)?;
                write!(output, "Введите букву: ")?;
                output.flush()?;
                line.clear();
                let count = Read::by_ref(input)
                    .take((MAX_LINE_BYTES + 1) as u64)
                    .read_until(b'\n', &mut line)?;

                if count == 0 {
                    writeln!(
                        output,
                        "\nВвод завершён. Игра прервана. Слово: {}",
                        game.secret_word()
                    )?;
                    return output.flush();
                }
                if line.len() > MAX_LINE_BYTES {
                    // Resynchronize at a line boundary; the remainder is not another guess.
                    if line.last() != Some(&b'\n') {
                        input.skip_until(b'\n')?;
                    }
                    writeln!(output, "Слишком длинная строка. Введите одну букву.")?;
                    continue;
                }
                let Ok(text) = std::str::from_utf8(&line) else {
                    writeln!(output, "Некорректный UTF-8. Введите одну букву.")?;
                    continue;
                };
                if game.submit_guess(text).is_err() {
                    writeln!(output, "Некорректный ввод. Введите ровно одну букву.")?;
                }
            }
            GameState::CheckingGuess(_) => {
                let result = game
                    .check_guess()
                    .expect("CheckingGuess always contains a validated pending letter");
                match result {
                    GuessResult::Hit => writeln!(output, "Есть такая буква!")?,
                    GuessResult::Miss => writeln!(output, "Такой буквы нет.")?,
                    GuessResult::Repeated => {
                        writeln!(output, "Эта буква уже введена. Попытка не потрачена.")?;
                    }
                }
            }
            GameState::Won => {
                show_state(output, &game)?;
                writeln!(
                    output,
                    "Вы выиграли! Правильное слово: {}",
                    game.secret_word()
                )?;
                return output.flush();
            }
            GameState::Lost => {
                show_state(output, &game)?;
                writeln!(
                    output,
                    "Вы проиграли. Правильное слово: {}",
                    game.secret_word()
                )?;
                return output.flush();
            }
        }
    }
}

fn show_state(output: &mut impl Write, game: &Game) -> io::Result<()> {
    writeln!(output, "\nСлово: {}", game.masked_word())?;
    write!(output, "Введённые буквы:")?;
    if game.guessed_letters().is_empty() {
        write!(output, " —")?;
    } else {
        for letter in game.guessed_letters() {
            write!(output, " {letter}")?;
        }
    }
    writeln!(output)?;
    writeln!(output, "Осталось попыток: {}", game.remaining_attempts())
}
