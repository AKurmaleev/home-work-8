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
                let text = match read_input_line(input, &mut line)? {
                    InputLine::Eof => {
                        writeln!(
                            output,
                            "\nВвод завершён. Игра прервана. Слово: {}",
                            game.secret_word()
                        )?;
                        return output.flush();
                    }
                    InputLine::Rejected(message) => {
                        writeln!(output, "{message}")?;
                        continue;
                    }
                    InputLine::Text(text) => text,
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

#[derive(Debug, Eq, PartialEq)]
enum InputLine<'a> {
    Eof,
    Rejected(&'static str),
    Text(&'a str),
}

// Text borrows the reusable buffer unchanged; letter validation belongs to Game.
fn read_input_line<'a>(
    input: &mut impl BufRead,
    line: &'a mut Vec<u8>,
) -> io::Result<InputLine<'a>> {
    line.clear();
    let count = Read::by_ref(input)
        .take((MAX_LINE_BYTES + 1) as u64)
        .read_until(b'\n', line)?;
    if count == 0 {
        return Ok(InputLine::Eof);
    }
    if line.len() > MAX_LINE_BYTES {
        // Resynchronize before returning; the remainder is not another guess.
        if line.last() != Some(&b'\n') {
            input.skip_until(b'\n')?;
        }
        return Ok(InputLine::Rejected(
            "Слишком длинная строка. Введите одну букву.",
        ));
    }
    Ok(match std::str::from_utf8(line) {
        Ok(text) => InputLine::Text(text),
        // Invalid encoding is a recoverable rejection with a fixed user message.
        Err(_) => InputLine::Rejected("Некорректный UTF-8. Введите одну букву."),
    })
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

#[cfg(test)]
mod input_tests {
    use std::io::{self, BufReader, Cursor, Read};

    use super::{InputLine, read_input_line};

    #[test]
    fn preserves_text_for_domain_validation_without_trimming_or_normalizing() {
        for text in [
            "\n",
            " \t\r\n",
            "ab\n",
            "exit\n",
            "İ\n",
            "e\u{301}\n",
            "\u{2003}Ё\r\n",
            "Ж",
        ] {
            let mut input = Cursor::new(text.as_bytes());
            let mut line = Vec::new();
            assert_eq!(
                read_input_line(&mut input, &mut line).unwrap(),
                InputLine::Text(text)
            );
        }
    }

    #[test]
    fn distinguishes_empty_line_from_eof_and_clears_reused_buffer() {
        let mut input = Cursor::new(b"\na");
        let mut line = b"stale bytes".to_vec();
        assert_eq!(
            read_input_line(&mut input, &mut line).unwrap(),
            InputLine::Text("\n")
        );
        assert_eq!(
            read_input_line(&mut input, &mut line).unwrap(),
            InputLine::Text("a")
        );
        for _ in 0..2 {
            assert_eq!(
                read_input_line(&mut input, &mut line).unwrap(),
                InputLine::Eof
            );
            assert!(line.is_empty());
        }
    }

    #[test]
    fn counts_utf8_and_crlf_bytes_at_the_128_byte_boundary() {
        for text in [
            format!("{}A", " ".repeat(127)),
            format!("{}Ё \r\n", "\u{2003}".repeat(41)),
        ] {
            assert_eq!(text.len(), 128);
            let mut input = Cursor::new(text.as_bytes());
            let mut line = Vec::new();
            assert_eq!(
                read_input_line(&mut input, &mut line).unwrap(),
                InputLine::Text(text.as_str())
            );
        }
        let mut input = Cursor::new(format!("{}Ё  \r\n", "\u{2003}".repeat(41)).into_bytes());
        let mut line = Vec::new();
        assert_eq!(
            read_input_line(&mut input, &mut line).unwrap(),
            InputLine::Rejected("Слишком длинная строка. Введите одну букву.")
        );
    }

    #[test]
    fn rejects_invalid_utf8_without_consuming_the_following_line() {
        for prefix in [&b"\xffr\n"[..], &b"\xc3\n"[..], &b"u\xff\n"[..]] {
            let mut input = Cursor::new([prefix, b"R\n"].concat());
            let mut line = Vec::new();
            assert_eq!(
                read_input_line(&mut input, &mut line).unwrap(),
                InputLine::Rejected("Некорректный UTF-8. Введите одну букву.")
            );
            assert_eq!(
                read_input_line(&mut input, &mut line).unwrap(),
                InputLine::Text("R\n")
            );
        }
    }

    #[test]
    fn discards_oversized_lines_once_without_retaining_the_remainder() {
        for mut prefix in [vec![b'x'; 128], vec![b'x'; 129], vec![b'\xff'; 1024 * 1024]] {
            prefix.push(b'\n');
            let expected_position = u64::try_from(prefix.len()).unwrap();
            prefix.extend_from_slice(b"r\n");
            let mut input = Cursor::new(prefix);
            let mut line = Vec::new();
            assert_eq!(
                read_input_line(&mut input, &mut line).unwrap(),
                InputLine::Rejected("Слишком длинная строка. Введите одну букву.")
            );
            assert_eq!(line.len(), 129);
            assert_eq!(input.position(), expected_position);
            assert_eq!(
                read_input_line(&mut input, &mut line).unwrap(),
                InputLine::Text("r\n")
            );
        }
        let mut input = Cursor::new(vec![b'x'; 130]);
        let mut line = Vec::new();
        assert_eq!(
            read_input_line(&mut input, &mut line).unwrap(),
            InputLine::Rejected("Слишком длинная строка. Введите одну букву.")
        );
        assert_eq!(
            read_input_line(&mut input, &mut line).unwrap(),
            InputLine::Eof
        );
    }

    #[test]
    fn handles_utf8_and_discard_boundaries_with_a_one_byte_buffer() {
        let bytes = [vec![b'x'; 129], "\nЁ\r\nЖ".as_bytes().to_vec()].concat();
        let mut input = BufReader::with_capacity(1, Cursor::new(bytes));
        let mut line = Vec::new();
        assert_eq!(
            read_input_line(&mut input, &mut line).unwrap(),
            InputLine::Rejected("Слишком длинная строка. Введите одну букву.")
        );
        assert_eq!(
            read_input_line(&mut input, &mut line).unwrap(),
            InputLine::Text("Ё\r\n")
        );
        assert_eq!(
            read_input_line(&mut input, &mut line).unwrap(),
            InputLine::Text("Ж")
        );
        assert_eq!(
            read_input_line(&mut input, &mut line).unwrap(),
            InputLine::Eof
        );
    }

    #[test]
    fn propagates_stream_errors_instead_of_reclassifying_them_as_rejections() {
        struct FailingReader(Cursor<Vec<u8>>);

        impl Read for FailingReader {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if buffer.is_empty() {
                    return Ok(0);
                }
                match self.0.read(buffer)? {
                    0 => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "injected stream failure",
                    )),
                    count => Ok(count),
                }
            }
        }

        for prefix in [Vec::new(), b"r".to_vec(), vec![b'x'; 129]] {
            let mut input = BufReader::with_capacity(2, FailingReader(Cursor::new(prefix)));
            let mut line = Vec::new();
            let error = read_input_line(&mut input, &mut line).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert_eq!(error.to_string(), "injected stream failure");
        }
    }
}
