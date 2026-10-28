use std::cell::RefCell;
use std::io::{self, BufRead, BufReader, Cursor, Read, Write};
use std::process::{Command, Stdio};
use std::rc::Rc;

use rust_homework_8::{
    console,
    game::{Game, GuessResult},
};

const PROMPT: &str = "Введите букву: ";
const HIT: &str = "Есть такая буква!";
const MISS: &str = "Такой буквы нет.";
const REPEATED: &str = "Эта буква уже введена.";
const INVALID: &str = "Некорректный ввод.";
const TOO_LONG: &str = "Слишком длинная строка.";

fn session(input: &[u8], word: &str) -> String {
    let mut input = Cursor::new(input);
    let mut output = Vec::new();
    console::run(
        &mut input,
        &mut output,
        Game::new(word, 5).expect("valid test game"),
    )
    .expect("in-memory console session succeeds");
    String::from_utf8(output).expect("console output is valid UTF-8")
}

fn fields<'a>(output: &'a str, prefix: &str) -> Vec<&'a str> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix(prefix))
        .collect()
}

fn masks(output: &str) -> Vec<&str> {
    fields(output, "Слово: ")
}

fn guesses(output: &str) -> Vec<&str> {
    fields(output, "Введённые буквы: ")
}

fn attempts(output: &str) -> Vec<u8> {
    fields(output, "Осталось попыток: ")
        .into_iter()
        .map(|value| value.parse().expect("numeric attempt count"))
        .collect()
}

fn assert_win(output: &str, word: &str, final_mask: &str, remaining: u8) {
    assert_eq!(output.matches("Вы выиграли!").count(), 1);
    assert!(output.ends_with(&format!("Правильное слово: {word}\n")));
    assert_eq!(masks(output).last().copied(), Some(final_mask));
    assert_eq!(attempts(output).last().copied(), Some(remaining));
    assert!(!output.contains("Вы проиграли"));
    assert!(!output.contains("Игра прервана"));
}

fn assert_interrupted(output: &str, word: &str) {
    assert_eq!(output.matches("Игра прервана.").count(), 1);
    assert!(output.ends_with(&format!("Игра прервана. Слово: {word}\n")));
    assert!(!output.contains("Вы выиграли"));
    assert!(!output.contains("Вы проиграли"));
    assert!(!output.contains("Правильное слово:"));
}

#[test]
fn rejected_input_preserves_exact_messages_before_unicode_win_or_eof() {
    let initial = concat!(
        "Игра «Виселица». Вводите по одной букве.\n",
        "\nСлово: _ _\n",
        "Введённые буквы: —\n",
        "Осталось попыток: 5\n",
        "Введите букву: ",
    );
    let waiting = concat!(
        "\nСлово: _ _\n",
        "Введённые буквы: —\n",
        "Осталось попыток: 5\n",
        "Введите букву: ",
    );
    let win = concat!(
        "Есть такая буква!\n",
        "\nСлово: ё _\n",
        "Введённые буквы: ё\n",
        "Осталось попыток: 5\n",
        "Введите букву: Есть такая буква!\n",
        "\nСлово: ё ж\n",
        "Введённые буквы: ё ж\n",
        "Осталось попыток: 5\n",
        "Вы выиграли! Правильное слово: ёж\n",
    );
    for (prefix, message) in [
        (
            vec![b'\xff', b'\n'],
            "Некорректный UTF-8. Введите одну букву.",
        ),
        (
            b"\n".to_vec(),
            "Некорректный ввод. Введите ровно одну букву.",
        ),
        (
            b"ab\n".to_vec(),
            "Некорректный ввод. Введите ровно одну букву.",
        ),
        (
            b"exit\n".to_vec(),
            "Некорректный ввод. Введите ровно одну букву.",
        ),
        (
            "İ\n".as_bytes().to_vec(),
            "Некорректный ввод. Введите ровно одну букву.",
        ),
        (
            [vec![b'x'; 129], b"\n".to_vec()].concat(),
            "Слишком длинная строка. Введите одну букву.",
        ),
    ] {
        assert_eq!(
            session(&prefix, "ёж"),
            format!("{initial}{message}\n{waiting}\nВвод завершён. Игра прервана. Слово: ёж\n"),
        );
        let mut input = prefix;
        input.extend_from_slice("\u{2003}Ё\r\nЖ".as_bytes());
        assert_eq!(
            session(&input, "ёж"),
            format!("{initial}{message}\n{waiting}{win}")
        );
    }
}

#[test]
fn win_shows_initial_and_final_masks_and_chronological_hits_without_penalty() {
    let output = session(b"t\ns\nu\nr\n", "rust");

    assert!(output.starts_with("Игра «Виселица»."));
    assert_eq!(
        masks(&output),
        ["_ _ _ _", "_ _ _ t", "_ _ s t", "_ u s t", "r u s t"]
    );
    assert_eq!(guesses(&output), ["—", "t", "t s", "t s u", "t s u r"]);
    assert_eq!(attempts(&output), [5; 5]);
    assert_eq!(output.matches(HIT).count(), 4);
    assert_eq!(output.matches(PROMPT).count(), 4);
    assert!(!output.contains(MISS));
    assert_win(&output, "rust", "r u s t", 5);
}

#[test]
fn five_distinct_misses_lose_reveal_the_word_and_leave_following_input_unread() {
    let mut input = Cursor::new(b"q\nw\ne\ny\ni\nr\nu\ns\nt\n");
    let mut output = Vec::new();
    console::run(&mut input, &mut output, Game::new("rust", 5).unwrap()).unwrap();
    let output = String::from_utf8(output).unwrap();

    assert_eq!(input.position(), 10);
    assert_eq!(masks(&output), ["_ _ _ _"; 6]);
    assert_eq!(
        guesses(&output),
        ["—", "q", "q w", "q w e", "q w e y", "q w e y i"]
    );
    assert_eq!(attempts(&output), [5, 4, 3, 2, 1, 0]);
    assert_eq!(output.matches(MISS).count(), 5);
    assert_eq!(output.matches(PROMPT).count(), 5);
    assert_eq!(output.matches("Вы проиграли.").count(), 1);
    assert!(output.ends_with("Вы проиграли. Правильное слово: rust\n"));
    assert!(!output.contains("Вы выиграли"));
    assert!(!output.contains("Игра прервана"));
}

#[test]
fn empty_multiletter_and_invalid_guesses_recover_without_recording_or_penalty() {
    for invalid in [
        "", " \t", "ru", "r u", "1", "!", "😀", "İ", "e\u{301}", "\0",
    ] {
        let input = format!("{invalid}\nr\nu\ns\nt\n");
        let output = session(input.as_bytes(), "rust");

        assert_eq!(output.matches(INVALID).count(), 1, "input: {invalid:?}");
        assert_eq!(attempts(&output), [5; 6], "input: {invalid:?}");
        assert_eq!(
            guesses(&output),
            ["—", "—", "r", "r u", "r u s", "r u s t"],
            "input: {invalid:?}"
        );
        assert_eq!(masks(&output)[..2], ["_ _ _ _"; 2]);
        assert_eq!(output.matches(HIT).count(), 4);
        assert!(!output.contains(MISS));
        assert_win(&output, "rust", "r u s t", 5);
    }
}

#[test]
fn repeated_hits_and_misses_preserve_state_and_unique_guess_chronology() {
    let output = session(b"r\nR\nx\nX\nu\ns\nt\n", "rust");

    assert_eq!(attempts(&output), [5, 5, 5, 4, 4, 4, 4, 4]);
    assert_eq!(
        guesses(&output),
        ["—", "r", "r", "r x", "r x", "r x u", "r x u s", "r x u s t"]
    );
    assert_eq!(
        masks(&output),
        [
            "_ _ _ _", "r _ _ _", "r _ _ _", "r _ _ _", "r _ _ _", "r u _ _", "r u s _", "r u s t"
        ]
    );
    assert_eq!(output.matches(REPEATED).count(), 2);
    assert_eq!(output.matches(MISS).count(), 1);
    assert_eq!(output.matches(HIT).count(), 4);
    assert_win(&output, "rust", "r u s t", 4);
}

#[test]
fn uppercase_cyrillic_crlf_guesses_reveal_all_occurrences_and_normalize_history() {
    let output = session(" \tЁ\r\nЖ\r\nЕ\r\n".as_bytes(), "ЁЖёЕ");

    assert_eq!(masks(&output), ["_ _ _ _", "ё _ ё _", "ё ж ё _", "ё ж ё е"]);
    assert_eq!(guesses(&output), ["—", "ё", "ё ж", "ё ж е"]);
    assert_eq!(attempts(&output), [5; 4]);
    assert_eq!(output.matches(HIT).count(), 3);
    assert!(!output.contains(INVALID));
    assert_win(&output, "ёжёе", "ё ж ё е", 5);
}

#[test]
fn immediate_eof_interrupts_with_initial_state_and_reveals_the_word() {
    let output = session(b"", "rust");

    assert_eq!(masks(&output), ["_ _ _ _"]);
    assert_eq!(guesses(&output), ["—"]);
    assert_eq!(attempts(&output), [5]);
    assert_eq!(output.matches(PROMPT).count(), 1);
    assert!(!output.contains(HIT));
    assert!(!output.contains(MISS));
    assert_interrupted(&output, "rust");
}

#[test]
fn eof_after_partial_progress_preserves_hits_misses_and_remaining_attempts() {
    let output = session(b"r\nx\nu\n", "rust");

    assert_eq!(masks(&output), ["_ _ _ _", "r _ _ _", "r _ _ _", "r u _ _"]);
    assert_eq!(guesses(&output), ["—", "r", "r x", "r x u"]);
    assert_eq!(attempts(&output), [5, 5, 4, 4]);
    assert_eq!(output.matches(PROMPT).count(), 4);
    assert_interrupted(&output, "rust");
}

#[test]
fn unterminated_nonwinning_guess_is_processed_before_eof_interrupts() {
    let output = session(b"r", "rust");

    assert_eq!(masks(&output), ["_ _ _ _", "r _ _ _"]);
    assert_eq!(guesses(&output), ["—", "r"]);
    assert_eq!(attempts(&output), [5, 5]);
    assert_eq!(output.matches(HIT).count(), 1);
    assert_interrupted(&output, "rust");
}

#[test]
fn final_winning_guess_without_newline_is_accepted() {
    let output = session(b"r\nu\ns\nt", "rust");

    assert_eq!(guesses(&output).last().copied(), Some("r u s t"));
    assert_eq!(attempts(&output), [5; 5]);
    assert_eq!(output.matches(HIT).count(), 4);
    assert_eq!(output.matches(PROMPT).count(), 4);
    assert_win(&output, "rust", "r u s t", 5);
}

#[test]
fn one_mebibyte_line_is_discarded_once_without_accepting_its_tail_or_next_line() {
    let mut bytes = vec![b'x'; 1024 * 1024];
    bytes.extend_from_slice(b"r\nr\nu\ns\nt\nunread\n");
    let expected_position = u64::try_from(bytes.len() - b"unread\n".len()).unwrap();
    let mut input = Cursor::new(bytes);
    let mut output = Vec::new();
    console::run(&mut input, &mut output, Game::new("rust", 5).unwrap()).unwrap();
    let output = String::from_utf8(output).unwrap();

    assert_eq!(input.position(), expected_position);
    assert_eq!(output.matches(TOO_LONG).count(), 1);
    assert_eq!(output.matches(PROMPT).count(), 5);
    assert_eq!(output.matches(HIT).count(), 4);
    assert!(!output.contains(INVALID));
    assert!(!output.contains(REPEATED));
    assert!(!output.contains(MISS));
    assert_eq!(attempts(&output), [5; 6]);
    assert_eq!(guesses(&output), ["—", "—", "r", "r u", "r u s", "r u s t"]);
    assert_eq!(masks(&output)[..2], ["_ _ _ _"; 2]);
    assert_win(&output, "rust", "r u s t", 5);
}

#[test]
fn line_of_exactly_128_bytes_including_newline_accepts_one_trimmed_letter() {
    let line = format!("{}r\n", " ".repeat(126));
    assert_eq!(line.len(), 128);
    let output = session(format!("{line}u\ns\nt\n").as_bytes(), "rust");

    assert_eq!(guesses(&output), ["—", "r", "r u", "r u s", "r u s t"]);
    assert_eq!(attempts(&output), [5; 5]);
    assert!(!output.contains(TOO_LONG));
    assert!(!output.contains(INVALID));
    assert_win(&output, "rust", "r u s t", 5);
}

#[test]
fn line_of_129_bytes_with_newline_already_consumed_does_not_skip_next_guess() {
    let line = format!("{}r\n", " ".repeat(127));
    assert_eq!(line.len(), 129);
    let output = session(format!("{line}r\nu\ns\nt\n").as_bytes(), "rust");

    assert_eq!(output.matches(TOO_LONG).count(), 1);
    assert_eq!(guesses(&output), ["—", "—", "r", "r u", "r u s", "r u s t"]);
    assert_eq!(attempts(&output), [5; 6]);
    assert!(!output.contains(REPEATED));
    assert_eq!(output.matches(HIT).count(), 4);
    assert_win(&output, "rust", "r u s t", 5);
}

#[test]
fn line_limit_counts_utf8_bytes_and_both_crlf_bytes_not_characters() {
    for (padding, length, oversized) in [(" ", 128, false), ("  ", 129, true)] {
        let line = format!("{}Ё{padding}\r\n", "\u{2003}".repeat(41));
        assert_eq!(line.len(), length);
        assert!(line.chars().count() < 128);
        let following = if oversized { "Ё\r\nЖ\r\n" } else { "Ж\r\n" };
        let output = session(format!("{line}{following}").as_bytes(), "ёж");

        assert_eq!(output.matches(TOO_LONG).count(), usize::from(oversized));
        assert_eq!(output.matches(HIT).count(), 2);
        assert!(!output.contains(INVALID));
        assert!(!output.contains(REPEATED));
        if oversized {
            assert_eq!(guesses(&output), ["—", "—", "ё", "ё ж"]);
            assert_eq!(attempts(&output), [5; 4]);
        } else {
            assert_eq!(guesses(&output), ["—", "ё", "ё ж"]);
            assert_eq!(attempts(&output), [5; 3]);
        }
        assert_win(&output, "ёж", "ё ж", 5);
    }
}

#[test]
fn invalid_utf8_lines_recover_and_do_not_accept_their_valid_letter_suffixes() {
    let input = b"\xffr\n\xc3\nu\xff\nr\nu\ns\nt\n";
    let output = session(input, "rust");

    assert_eq!(output.matches("Некорректный UTF-8.").count(), 3);
    assert_eq!(
        guesses(&output),
        ["—", "—", "—", "—", "r", "r u", "r u s", "r u s t"]
    );
    assert_eq!(attempts(&output), [5; 8]);
    assert_eq!(output.matches(HIT).count(), 4);
    assert!(!output.contains(REPEATED));
    assert!(!output.contains(MISS));
    assert_win(&output, "rust", "r u s t", 5);
}

#[test]
fn oversized_unterminated_line_is_discarded_through_eof_then_interrupts() {
    let output = session(&vec![b'x'; 1024], "rust");

    assert_eq!(output.matches(TOO_LONG).count(), 1);
    assert_eq!(guesses(&output), ["—", "—"]);
    assert_eq!(attempts(&output), [5, 5]);
    assert!(!output.contains(MISS));
    assert_interrupted(&output, "rust");
}

struct ErrorAfterPrefix {
    prefix: Cursor<Vec<u8>>,
}

impl Read for ErrorAfterPrefix {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let count = self.prefix.read(buffer)?;
        if count == 0 {
            Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "injected read failure",
            ))
        } else {
            Ok(count)
        }
    }
}

#[test]
fn reader_error_after_partial_line_propagates_without_applying_partial_guess() {
    let mut input = BufReader::with_capacity(
        2,
        ErrorAfterPrefix {
            prefix: Cursor::new(b"r\nu".to_vec()),
        },
    );
    let mut output = Vec::new();
    let error = console::run(&mut input, &mut output, Game::new("rust", 5).unwrap())
        .expect_err("reader failure must propagate");
    let output = String::from_utf8(output).unwrap();

    assert_eq!(error.kind(), io::ErrorKind::ConnectionReset);
    assert_eq!(error.to_string(), "injected read failure");
    assert_eq!(output.matches(HIT).count(), 1);
    assert_eq!(guesses(&output), ["—", "r"]);
    assert_eq!(masks(&output), ["_ _ _ _", "r _ _ _"]);
    assert!(!output.contains("Игра прервана"));
    assert!(!output.contains("Правильное слово:"));
}

#[test]
fn reader_error_while_discarding_oversized_line_propagates() {
    let mut input = BufReader::with_capacity(
        16,
        ErrorAfterPrefix {
            prefix: Cursor::new(vec![b'x'; 129]),
        },
    );
    let mut output = Vec::new();
    let error = console::run(&mut input, &mut output, Game::new("rust", 5).unwrap())
        .expect_err("discard read failure must propagate");
    let output = String::from_utf8(output).unwrap();

    assert_eq!(error.kind(), io::ErrorKind::ConnectionReset);
    assert_eq!(error.to_string(), "injected read failure");
    assert_eq!(guesses(&output), ["—"]);
    assert_eq!(attempts(&output), [5]);
    assert!(!output.contains(MISS));
    assert!(!output.contains("Игра прервана"));
}

#[derive(Default)]
struct OutputRecord {
    bytes: Vec<u8>,
    flush_lengths: Vec<usize>,
}

#[derive(Default)]
struct ObservedWriter {
    record: Rc<RefCell<OutputRecord>>,
    write_limit: Option<usize>,
    fail_flush: Option<usize>,
}

impl Write for ObservedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let mut record = self.record.borrow_mut();
        let count = match self.write_limit {
            Some(limit) => {
                if record.bytes.len() >= limit {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "injected write failure",
                    ));
                }
                buffer.len().min(limit - record.bytes.len())
            }
            None => buffer.len(),
        };
        record.bytes.extend_from_slice(&buffer[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut record = self.record.borrow_mut();
        let length = record.bytes.len();
        record.flush_lengths.push(length);
        if self.fail_flush == Some(record.flush_lengths.len()) {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "injected flush failure",
            ))
        } else {
            Ok(())
        }
    }
}

#[test]
fn writer_failure_after_short_write_propagates_before_reading_input() {
    let mut input = Cursor::new(b"r\nu\ns\nt\n");
    let mut output = ObservedWriter {
        write_limit: Some(7),
        ..ObservedWriter::default()
    };
    let error = console::run(&mut input, &mut output, Game::new("rust", 5).unwrap())
        .expect_err("write failure must propagate");

    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(error.to_string(), "injected write failure");
    assert_eq!(input.position(), 0);
    assert_eq!(
        output.record.borrow().bytes,
        "Игра «Виселица».".as_bytes()[..7]
    );
    assert!(output.record.borrow().flush_lengths.is_empty());
}

#[test]
fn prompt_flush_failure_propagates_without_consuming_input() {
    let mut input = Cursor::new(b"r\nu\ns\nt\n");
    let mut output = ObservedWriter {
        fail_flush: Some(1),
        ..ObservedWriter::default()
    };
    let error = console::run(&mut input, &mut output, Game::new("rust", 5).unwrap())
        .expect_err("prompt flush failure must propagate");

    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(error.to_string(), "injected flush failure");
    assert_eq!(input.position(), 0);
    let record = output.record.borrow();
    assert!(record.bytes.ends_with(PROMPT.as_bytes()));
    assert_eq!(record.flush_lengths, [record.bytes.len()]);
}

#[test]
fn final_flush_failure_propagates_for_win_loss_and_eof() {
    for (input, final_flush, ending) in [
        (
            &b"r\nu\ns\nt\n"[..],
            5,
            "Вы выиграли! Правильное слово: rust\n",
        ),
        (
            &b"q\nw\ne\ny\ni\n"[..],
            6,
            "Вы проиграли. Правильное слово: rust\n",
        ),
        (&b""[..], 2, "Игра прервана. Слово: rust\n"),
    ] {
        let mut input = Cursor::new(input);
        let mut output = ObservedWriter {
            fail_flush: Some(final_flush),
            ..ObservedWriter::default()
        };
        let error = console::run(&mut input, &mut output, Game::new("rust", 5).unwrap())
            .expect_err("final flush failure must not become success");

        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(error.to_string(), "injected flush failure");
        let record = output.record.borrow();
        assert!(record.bytes.ends_with(ending.as_bytes()));
        assert_eq!(record.flush_lengths.len(), final_flush);
        assert_eq!(record.flush_lengths.last(), Some(&record.bytes.len()));
    }
}

#[test]
fn successful_sessions_flush_final_output_for_win_loss_and_eof() {
    for (input, expected_flushes, ending) in [
        (
            &b"r\nu\ns\nt\n"[..],
            5,
            "Вы выиграли! Правильное слово: rust\n",
        ),
        (
            &b"q\nw\ne\ny\ni\n"[..],
            6,
            "Вы проиграли. Правильное слово: rust\n",
        ),
        (&b"r\n"[..], 3, "Игра прервана. Слово: rust\n"),
    ] {
        let mut input = Cursor::new(input);
        let mut output = ObservedWriter::default();
        console::run(&mut input, &mut output, Game::new("rust", 5).unwrap()).unwrap();

        let record = output.record.borrow();
        assert!(record.bytes.ends_with(ending.as_bytes()));
        assert_eq!(record.flush_lengths.len(), expected_flushes);
        assert_eq!(record.flush_lengths.last(), Some(&record.bytes.len()));
    }
}

struct PromptCheckingReader {
    input: Cursor<Vec<u8>>,
    output: Rc<RefCell<OutputRecord>>,
    checks: usize,
}

impl Read for PromptCheckingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let available = self.fill_buf()?;
        let count = available.len().min(buffer.len());
        buffer[..count].copy_from_slice(&available[..count]);
        self.consume(count);
        Ok(count)
    }
}

impl BufRead for PromptCheckingReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        let record = self.output.borrow();
        assert!(
            record.bytes.ends_with(PROMPT.as_bytes()),
            "read must follow a prompt"
        );
        assert_eq!(
            record.flush_lengths.last(),
            Some(&record.bytes.len()),
            "the complete prompt must be flushed before input is read"
        );
        self.checks += 1;
        self.input.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.input.consume(amount);
    }
}

#[test]
fn every_prompt_is_flushed_before_input_is_read() {
    let mut output = ObservedWriter::default();
    let mut input = PromptCheckingReader {
        input: Cursor::new(b"r\nu\ns\nt\n".to_vec()),
        output: Rc::clone(&output.record),
        checks: 0,
    };
    console::run(&mut input, &mut output, Game::new("rust", 5).unwrap()).unwrap();

    assert!(input.checks >= 4);
    assert_eq!(input.input.position(), 8);
    let record = output.record.borrow();
    let text = std::str::from_utf8(&record.bytes).unwrap();
    assert_eq!(text.matches(PROMPT).count(), 4);
    assert_eq!(record.flush_lengths.len(), 5);
    assert_win(text, "rust", "r u s t", 5);
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum CleanupStep {
    Kill,
    Wait,
}

fn cli_input_failure(
    error: io::Error,
    mut cleanup: impl FnMut(CleanupStep) -> io::Result<()>,
) -> io::Error {
    // Preserve the input failure and attempt both steps before reporting cleanup outcomes.
    let kill = cleanup(CleanupStep::Kill);
    let wait = cleanup(CleanupStep::Wait);
    io::Error::new(
        error.kind(),
        format!("cannot write finite CLI input: {error}; kill: {kill:?}; wait: {wait:?}"),
    )
}

#[test]
fn cli_cleanup_attempts_both_steps_and_retains_all_failures() {
    for (kill_fails, wait_fails) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut steps = Vec::new();
        let failure = cli_input_failure(
            io::Error::new(io::ErrorKind::BrokenPipe, "injected input failure"),
            |step| {
                steps.push(step);
                match step {
                    CleanupStep::Kill if kill_fails => Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "injected kill failure",
                    )),
                    CleanupStep::Wait if wait_fails => {
                        Err(io::Error::other("injected wait failure"))
                    }
                    CleanupStep::Kill | CleanupStep::Wait => Ok(()),
                }
            },
        );

        assert_eq!(steps, [CleanupStep::Kill, CleanupStep::Wait]);
        assert_eq!(failure.kind(), io::ErrorKind::BrokenPipe);
        let message = failure.to_string();
        assert!(message.starts_with("cannot write finite CLI input: injected input failure"));
        assert_eq!(message.contains("injected kill failure"), kill_fails);
        assert_eq!(message.contains("injected wait failure"), wait_fails);
    }
}

#[test]
fn cli_main_runs_the_fixed_rust_game_with_five_attempts() {
    // The caller must bound the overall test command: wait_with_output has no timeout.
    let mut child = Command::new(env!("CARGO_BIN_EXE_rust_homework_8"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("rust_homework_8 executable starts");
    let write_result = match child.stdin.take() {
        Some(mut stdin) => stdin.write_all("\u{2003}R\r\nU\r\nS\r\nT".as_bytes()),
        None => Err(io::Error::other("child stdin pipe missing")),
    };
    if let Err(error) = write_result {
        let failure = cli_input_failure(error, |step| match step {
            CleanupStep::Kill => child.kill(),
            CleanupStep::Wait => child.wait().map(|_| ()),
        });
        panic!("{failure}");
    }
    let output = child
        .wait_with_output()
        .expect("CLI exits and output is collected");

    assert!(
        output.status.success(),
        "CLI status: {:?}, stderr: {:?}",
        output.status,
        output.stderr
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let output = String::from_utf8(output.stdout).expect("CLI emits valid UTF-8");
    assert_eq!(masks(&output).first().copied(), Some("_ _ _ _"));
    assert_eq!(guesses(&output), ["—", "r", "r u", "r u s", "r u s t"]);
    assert_eq!(attempts(&output), [5; 5]);
    assert_eq!(output.matches(PROMPT).count(), 4);
    assert_eq!(output.matches(HIT).count(), 4);
    assert!(!output.contains(INVALID));
    assert_win(&output, "rust", "r u s t", 5);
}

#[test]
fn pending_winning_guess_is_checked_without_reading_or_prompting() {
    let mut game = Game::new("Aa", 1).unwrap();
    game.submit_guess("A").unwrap();
    let mut input = Cursor::new(b"unread\n");
    let mut output = ObservedWriter::default();

    console::run(&mut input, &mut output, game).unwrap();

    assert_eq!(input.position(), 0);
    let record = output.record.borrow();
    let text = std::str::from_utf8(&record.bytes).unwrap();
    assert_eq!(masks(text), ["a a"]);
    assert_eq!(guesses(text), ["a"]);
    assert_eq!(text.matches(HIT).count(), 1);
    assert!(!text.contains(PROMPT));
    assert_eq!(record.flush_lengths, [record.bytes.len()]);
    assert_win(text, "aa", "a a", 1);
}

#[test]
fn pending_losing_guess_is_checked_without_reading_or_prompting() {
    let mut game = Game::new("a", 1).unwrap();
    game.submit_guess("X").unwrap();
    let mut input = Cursor::new(b"a\n");
    let mut output = ObservedWriter::default();

    console::run(&mut input, &mut output, game).unwrap();

    assert_eq!(input.position(), 0);
    let record = output.record.borrow();
    let text = std::str::from_utf8(&record.bytes).unwrap();
    assert_eq!(masks(text), ["_"]);
    assert_eq!(guesses(text), ["x"]);
    assert_eq!(attempts(text), [0]);
    assert_eq!(text.matches(MISS).count(), 1);
    assert_eq!(text.matches("Вы проиграли.").count(), 1);
    assert!(text.ends_with("Вы проиграли. Правильное слово: a\n"));
    assert!(!text.contains(PROMPT));
    assert_eq!(record.flush_lengths, [record.bytes.len()]);
}

#[test]
fn pending_nonterminal_guess_is_checked_before_the_first_flushed_prompt() {
    let mut game = Game::new("ab", 2).unwrap();
    game.submit_guess("A").unwrap();
    let mut output = ObservedWriter::default();
    let mut input = PromptCheckingReader {
        input: Cursor::new(b"b\nunread\n".to_vec()),
        output: Rc::clone(&output.record),
        checks: 0,
    };

    console::run(&mut input, &mut output, game).unwrap();

    assert_eq!(input.input.position(), 2);
    assert!(input.checks > 0);
    let record = output.record.borrow();
    let text = std::str::from_utf8(&record.bytes).unwrap();
    assert_eq!(masks(text), ["a _", "a b"]);
    assert_eq!(guesses(text), ["a", "a b"]);
    assert_eq!(text.matches(HIT).count(), 2);
    assert_eq!(text.matches(PROMPT).count(), 1);
    assert_eq!(record.flush_lengths.len(), 2);
    assert!(text.find(HIT).unwrap() < text.find(PROMPT).unwrap());
    assert_win(text, "ab", "a b", 2);
}

#[test]
fn pending_repeat_returns_to_waiting_without_recording_or_penalty() {
    let mut game = Game::new("ab", 2).unwrap();
    game.submit_guess("x").unwrap();
    assert_eq!(game.check_guess(), Some(GuessResult::Miss));
    game.submit_guess("X").unwrap();
    let mut input = Cursor::new(b"a\nb\nunread\n");
    let mut output = Vec::new();

    console::run(&mut input, &mut output, game).unwrap();

    assert_eq!(input.position(), 4);
    let text = String::from_utf8(output).unwrap();
    assert_eq!(guesses(&text), ["x", "x a", "x a b"]);
    assert_eq!(attempts(&text), [1; 3]);
    assert_eq!(text.matches(REPEATED).count(), 1);
    assert!(!text.contains(MISS));
    assert_win(&text, "ab", "a b", 1);
}

#[test]
fn already_terminal_games_never_read_even_from_a_failing_reader() {
    for (letter, result, ending, mask, remaining) in [
        (
            "a",
            GuessResult::Hit,
            "Вы выиграли! Правильное слово: a\n",
            "a",
            1,
        ),
        (
            "x",
            GuessResult::Miss,
            "Вы проиграли. Правильное слово: a\n",
            "_",
            0,
        ),
    ] {
        let mut game = Game::new("a", 1).unwrap();
        game.submit_guess(letter).unwrap();
        assert_eq!(game.check_guess(), Some(result));
        let mut input = BufReader::new(ErrorAfterPrefix {
            prefix: Cursor::new(Vec::new()),
        });
        let mut output = ObservedWriter::default();

        console::run(&mut input, &mut output, game).unwrap();

        let record = output.record.borrow();
        let text = std::str::from_utf8(&record.bytes).unwrap();
        assert_eq!(masks(text), [mask]);
        assert_eq!(guesses(text), [letter]);
        assert_eq!(attempts(text), [remaining]);
        assert!(text.ends_with(ending));
        assert!(!text.contains(PROMPT));
        assert!(!text.contains(HIT));
        assert!(!text.contains(MISS));
        assert_eq!(record.flush_lengths, [record.bytes.len()]);
    }
}

#[test]
fn state_machine_preserves_the_exact_single_letter_win_transcript() {
    assert_eq!(
        session(b"A\n", "a"),
        concat!(
            "Игра «Виселица». Вводите по одной букве.\n",
            "\nСлово: _\n",
            "Введённые буквы: —\n",
            "Осталось попыток: 5\n",
            "Введите букву: Есть такая буква!\n",
            "\nСлово: a\n",
            "Введённые буквы: a\n",
            "Осталось попыток: 5\n",
            "Вы выиграли! Правильное слово: a\n",
        )
    );
}

#[test]
fn exactly_128_unterminated_bytes_are_accepted_before_terminal_output() {
    let input = format!("{}A", " ".repeat(127));
    assert_eq!(input.len(), 128);
    let output = session(input.as_bytes(), "a");

    assert!(!output.contains(TOO_LONG));
    assert_eq!(output.matches(PROMPT).count(), 1);
    assert_win(&output, "a", "a", 5);
}

#[test]
fn single_byte_buffer_preserves_multibyte_utf8_crlf_and_discard_boundaries() {
    let bytes = format!("{}\nЁ\r\nЖ\r\nunread\n", "x".repeat(129)).into_bytes();
    let mut input = BufReader::with_capacity(1, Cursor::new(bytes));
    let mut output = Vec::new();

    console::run(&mut input, &mut output, Game::new("ёж", 5).unwrap()).unwrap();

    let text = String::from_utf8(output).unwrap();
    assert_eq!(text.matches(TOO_LONG).count(), 1);
    assert!(!text.contains("Некорректный UTF-8."));
    assert_eq!(guesses(&text), ["—", "—", "ё", "ё ж"]);
    assert_eq!(attempts(&text), [5; 4]);
    assert_win(&text, "ёж", "ё ж", 5);
    let mut remainder = Vec::new();
    input.read_to_end(&mut remainder).unwrap();
    assert_eq!(remainder, b"unread\n");
}
