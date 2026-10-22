use rust_homework_8::game::{Game, GameError, GameState, GuessResult};

fn assert_state(
    game: &Game,
    state: GameState,
    attempts: u8,
    guessed_letters: &[char],
    masked_word: &str,
    secret_word: &str,
) {
    assert_eq!(game.state(), state);
    assert_eq!(game.remaining_attempts(), attempts);
    assert_eq!(game.guessed_letters(), guessed_letters);
    assert_eq!(game.masked_word(), masked_word);
    assert_eq!(game.secret_word(), secret_word);
}

fn guess(game: &mut Game, input: &str) -> Result<GuessResult, GameError> {
    game.submit_guess(input)?;
    Ok(game.check_guess().expect("submitted guess is pending"))
}

#[test]
fn new_game_hides_every_position_and_normalizes_the_word() {
    let game = Game::new("RuSt", 6).expect("valid word and attempts");

    assert_state(&game, GameState::WaitingForInput, 6, &[], "_ _ _ _", "rust");
}

#[test]
fn game_owns_its_word_independently_of_constructor_input() {
    let mut game = {
        let word = String::from("Ab");
        Game::new(&word, 2).expect("valid borrowed word")
    };

    assert_eq!(guess(&mut game, "A"), Ok(GuessResult::Hit));
    assert_state(&game, GameState::WaitingForInput, 2, &['a'], "a _", "ab");
}

#[test]
fn submission_exposes_normalized_pending_letter_without_applying_it() {
    let mut game = Game::new("ЁЖё", 2).unwrap();

    assert_eq!(game.submit_guess("\u{2003}Ё\r\n"), Ok(()));
    assert_state(&game, GameState::CheckingGuess('ё'), 2, &[], "_ _ _", "ёжё");
    let state = game.state();
    let copied_state = state;
    assert_eq!(state, copied_state);
    assert_eq!(game.state(), state);

    assert_eq!(game.check_guess(), Some(GuessResult::Hit));
    assert_state(&game, GameState::WaitingForInput, 2, &['ё'], "ё _ ё", "ёжё");
}

#[test]
fn pending_guess_cannot_be_replaced_even_by_invalid_input() {
    let mut game = Game::new("ab", 2).unwrap();
    game.submit_guess("A").unwrap();

    for input in ["b", "x", "A", "", "ab", "İ", "😀"] {
        assert_eq!(game.submit_guess(input), Err(GameError::NotWaitingForInput));
        assert_state(&game, GameState::CheckingGuess('a'), 2, &[], "_ _", "ab");
    }
    assert_eq!(game.check_guess(), Some(GuessResult::Hit));
    assert_state(&game, GameState::WaitingForInput, 2, &['a'], "a _", "ab");
}

#[test]
fn checking_without_pending_input_is_a_no_op_before_and_after_a_guess() {
    let mut game = Game::new("ab", 2).unwrap();

    for _ in 0..2 {
        assert_eq!(game.check_guess(), None);
        assert_state(&game, GameState::WaitingForInput, 2, &[], "_ _", "ab");
    }
    game.submit_guess("x").unwrap();
    assert_eq!(game.check_guess(), Some(GuessResult::Miss));
    for _ in 0..2 {
        assert_eq!(game.check_guess(), None);
        assert_state(&game, GameState::WaitingForInput, 1, &['x'], "_ _", "ab");
    }
}

#[test]
fn winning_guess_remains_pending_until_checked() {
    let mut game = Game::new("Aa", 1).unwrap();

    game.submit_guess("A").unwrap();
    assert_state(&game, GameState::CheckingGuess('a'), 1, &[], "_ _", "aa");
    assert_eq!(game.check_guess(), Some(GuessResult::Hit));
    assert_state(&game, GameState::Won, 1, &['a'], "a a", "aa");
}

#[test]
fn losing_guess_remains_pending_until_checked() {
    let mut game = Game::new("a", 1).unwrap();

    game.submit_guess("X").unwrap();
    assert_state(&game, GameState::CheckingGuess('x'), 1, &[], "_", "a");
    assert_eq!(game.check_guess(), Some(GuessResult::Miss));
    assert_state(&game, GameState::Lost, 0, &['x'], "_", "a");
}

#[test]
fn repeated_guess_has_its_own_pending_phase_without_a_second_penalty() {
    let mut game = Game::new("aba", 2).unwrap();
    assert_eq!(guess(&mut game, "x"), Ok(GuessResult::Miss));
    assert_eq!(guess(&mut game, "a"), Ok(GuessResult::Hit));

    for (input, letter) in [("X", 'x'), ("A", 'a')] {
        game.submit_guess(input).unwrap();
        assert_state(
            &game,
            GameState::CheckingGuess(letter),
            1,
            &['x', 'a'],
            "a _ a",
            "aba",
        );
        assert_eq!(game.check_guess(), Some(GuessResult::Repeated));
        assert_state(
            &game,
            GameState::WaitingForInput,
            1,
            &['x', 'a'],
            "a _ a",
            "aba",
        );
    }
}

#[test]
fn hit_reveals_all_occurrences_including_first_and_last_positions() {
    let mut game = Game::new("BananaB", 3).expect("valid word");

    assert_eq!(guess(&mut game, "b"), Ok(GuessResult::Hit));
    assert_eq!(game.masked_word(), "b _ _ _ _ _ b");
    assert_eq!(guess(&mut game, "A"), Ok(GuessResult::Hit));
    assert_state(
        &game,
        GameState::WaitingForInput,
        3,
        &['b', 'a'],
        "b a _ a _ a b",
        "bananab",
    );
}

#[test]
fn cyrillic_and_yo_match_in_both_cases_without_merging_e_and_yo() {
    let mut game = Game::new("ЁЖёЕ", 3).expect("valid Cyrillic word");

    assert_eq!(guess(&mut game, "Ё"), Ok(GuessResult::Hit));
    assert_state(
        &game,
        GameState::WaitingForInput,
        3,
        &['ё'],
        "ё _ ё _",
        "ёжёе",
    );
    assert_eq!(guess(&mut game, "ё"), Ok(GuessResult::Repeated));
    assert_eq!(guess(&mut game, "ж"), Ok(GuessResult::Hit));
    assert_eq!(guess(&mut game, "Е"), Ok(GuessResult::Hit));
    assert_state(
        &game,
        GameState::Won,
        3,
        &['ё', 'ж', 'е'],
        "ё ж ё е",
        "ёжёе",
    );
}

#[test]
fn repeated_hits_including_uppercase_leave_checked_state_unchanged() {
    let mut game = Game::new("aba", 2).expect("valid word");
    assert_eq!(guess(&mut game, "a"), Ok(GuessResult::Hit));

    for input in ["a", "A", " A "] {
        assert_eq!(guess(&mut game, input), Ok(GuessResult::Repeated));
        assert_state(&game, GameState::WaitingForInput, 2, &['a'], "a _ a", "aba");
    }
}

#[test]
fn repeated_misses_including_uppercase_do_not_consume_more_attempts() {
    let mut game = Game::new("ab", 2).expect("valid word");
    assert_eq!(guess(&mut game, "X"), Ok(GuessResult::Miss));

    for input in ["x", "X", " x "] {
        assert_eq!(guess(&mut game, input), Ok(GuessResult::Repeated));
        assert_state(&game, GameState::WaitingForInput, 1, &['x'], "_ _", "ab");
    }
}

#[test]
fn distinct_hits_and_misses_are_not_repeats_and_preserve_chronology() {
    let mut game = Game::new("abc", 4).expect("valid word");

    for (input, result) in [
        ("X", GuessResult::Miss),
        ("B", GuessResult::Hit),
        ("y", GuessResult::Miss),
        ("a", GuessResult::Hit),
        ("b", GuessResult::Repeated),
    ] {
        assert_eq!(guess(&mut game, input), Ok(result));
    }
    assert_state(
        &game,
        GameState::WaitingForInput,
        2,
        &['x', 'b', 'y', 'a'],
        "a b _",
        "abc",
    );
}

#[test]
fn guesses_trim_ascii_and_unicode_whitespace() {
    let mut game = Game::new("ab", 2).expect("valid word");

    assert_eq!(guess(&mut game, " \tA\r\n"), Ok(GuessResult::Hit));
    assert_eq!(guess(&mut game, "\u{2003}B\u{a0}"), Ok(GuessResult::Hit));
    assert_state(&game, GameState::Won, 2, &['a', 'b'], "a b", "ab");
}

#[test]
fn invalid_guesses_do_not_mutate_any_observable_state() {
    let mut game = Game::new("ab", 3).expect("valid word");
    assert_eq!(guess(&mut game, "A"), Ok(GuessResult::Hit));
    assert_eq!(guess(&mut game, "x"), Ok(GuessResult::Miss));

    for input in [
        "", " \t\r\n", "\u{2003}", "ab", "a b", "a\nb", "!", "_", "1", "٣", "😀", "e\u{301}",
        "\u{301}", "\0", "a!",
    ] {
        assert_eq!(
            game.submit_guess(input),
            Err(GameError::InvalidGuess),
            "input: {input:?}"
        );
        assert_state(
            &game,
            GameState::WaitingForInput,
            2,
            &['a', 'x'],
            "a _",
            "ab",
        );
        assert_eq!(game.check_guess(), None);
    }

    assert_eq!(guess(&mut game, "b"), Ok(GuessResult::Hit));
    assert_eq!(game.state(), GameState::Won);
}

#[test]
fn expanding_lowercase_guess_is_rejected_without_recording_partial_mapping() {
    let mut game = Game::new("ib", 2).expect("valid word");

    assert_eq!(game.submit_guess("İ"), Err(GameError::InvalidGuess));
    assert_state(&game, GameState::WaitingForInput, 2, &[], "_ _", "ib");
    assert_eq!(guess(&mut game, "I"), Ok(GuessResult::Hit));
    assert_state(&game, GameState::WaitingForInput, 2, &['i'], "i _", "ib");
}

#[test]
fn win_requires_every_position_and_hits_preserve_the_last_attempt() {
    let mut game = Game::new("aba", 1).expect("one attempt is valid");

    assert_eq!(guess(&mut game, "a"), Ok(GuessResult::Hit));
    assert_eq!(game.state(), GameState::WaitingForInput);
    assert_eq!(guess(&mut game, "b"), Ok(GuessResult::Hit));
    assert_state(&game, GameState::Won, 1, &['a', 'b'], "a b a", "aba");
}

#[test]
fn loss_occurs_exactly_when_the_last_attempt_is_consumed() {
    let mut game = Game::new("ab", 2).expect("valid word");

    assert_eq!(guess(&mut game, "a"), Ok(GuessResult::Hit));
    assert_eq!(guess(&mut game, "x"), Ok(GuessResult::Miss));
    assert_state(
        &game,
        GameState::WaitingForInput,
        1,
        &['a', 'x'],
        "a _",
        "ab",
    );
    assert_eq!(guess(&mut game, "y"), Ok(GuessResult::Miss));
    assert_state(&game, GameState::Lost, 0, &['a', 'x', 'y'], "a _", "ab");
}

#[test]
fn won_game_rejects_valid_repeated_and_invalid_input_without_mutation() {
    let mut game = Game::new("A", 1).expect("single-letter word is valid");
    assert_eq!(guess(&mut game, "a"), Ok(GuessResult::Hit));

    for input in ["b", "A", "", "ab", "!", "😀", "İ"] {
        assert_eq!(game.submit_guess(input), Err(GameError::NotWaitingForInput));
        assert_eq!(game.check_guess(), None);
        assert_state(&game, GameState::Won, 1, &['a'], "a", "a");
    }
}

#[test]
fn lost_game_rejects_hits_repeats_and_invalid_input_without_underflow() {
    let mut game = Game::new("A", 1).expect("single attempt is valid");
    assert_eq!(guess(&mut game, "x"), Ok(GuessResult::Miss));

    for input in ["a", "x", "X", "y", "", "ab", "!", "😀", "İ"] {
        assert_eq!(game.submit_guess(input), Err(GameError::NotWaitingForInput));
        assert_eq!(game.check_guess(), None);
        assert_state(&game, GameState::Lost, 0, &['x'], "_", "a");
    }
}

#[test]
fn maximum_u8_attempt_count_is_accepted_and_decrements_by_one() {
    let mut game = Game::new("ab", u8::MAX).expect("maximum attempts is valid");
    assert_eq!(game.remaining_attempts(), u8::MAX);

    assert_eq!(guess(&mut game, "x"), Ok(GuessResult::Miss));
    assert_state(&game, GameState::WaitingForInput, 254, &['x'], "_ _", "ab");
}

#[test]
fn constructor_rejects_empty_word() {
    assert!(matches!(Game::new("", 2), Err(GameError::InvalidWord)));
}

#[test]
fn constructor_rejects_nonalphabetic_words_without_trimming() {
    for word in [
        " ", " ab", "ab ", "a b", "a-b", "a1", "😀", "e\u{301}", "a\0",
    ] {
        assert!(
            matches!(Game::new(word, 2), Err(GameError::InvalidWord)),
            "word: {word:?}"
        );
    }
}

#[test]
fn constructor_rejects_expanding_lowercase_anywhere_in_word() {
    for word in ["İ", "aİ", "İa"] {
        assert!(matches!(Game::new(word, 2), Err(GameError::InvalidWord)));
    }
}

#[test]
fn constructor_rejects_zero_attempts() {
    assert!(matches!(
        Game::new("rust", 0),
        Err(GameError::InvalidAttempts)
    ));
}

#[test]
fn constructor_rejects_words_longer_than_64_scalars() {
    for letter in ["a", "Ё"] {
        let word = letter.repeat(65);
        assert!(matches!(Game::new(&word, 2), Err(GameError::InvalidWord)));
    }
}

#[test]
fn exactly_64_scalars_are_accepted_even_when_utf8_uses_more_bytes() {
    for (uppercase, lowercase) in [("A", "a"), ("Ё", "ё")] {
        let word = uppercase.repeat(64);
        let mut game = Game::new(&word, 1).expect("64 scalars are valid");
        let hidden = vec!["_"; 64].join(" ");
        assert_state(
            &game,
            GameState::WaitingForInput,
            1,
            &[],
            &hidden,
            &lowercase.repeat(64),
        );

        assert_eq!(guess(&mut game, uppercase), Ok(GuessResult::Hit));
        assert_eq!(game.state(), GameState::Won);
        assert_eq!(game.remaining_attempts(), 1);
        assert_eq!(game.masked_word(), vec![lowercase; 64].join(" "));
        assert_eq!(game.guessed_letters().len(), 1);
    }
}

#[test]
fn precomposed_uncased_and_supplementary_alphabetic_scalars_are_supported() {
    let mut game = Game::new("É山𐐀", 2).expect("valid Unicode letters");

    for input in ["é", "山", "𐐨"] {
        assert_eq!(guess(&mut game, input), Ok(GuessResult::Hit));
    }
    assert_state(
        &game,
        GameState::Won,
        2,
        &['é', '山', '𐐨'],
        "é 山 𐐨",
        "é山𐐨",
    );
}

#[test]
fn lowercase_matching_does_not_apply_full_case_folding() {
    let mut game = Game::new("ßs", 2).expect("valid lowercase mappings");

    assert_eq!(guess(&mut game, "ẞ"), Ok(GuessResult::Hit));
    assert_state(&game, GameState::WaitingForInput, 2, &['ß'], "ß _", "ßs");
    assert_eq!(guess(&mut game, "S"), Ok(GuessResult::Hit));
    assert_state(&game, GameState::Won, 2, &['ß', 's'], "ß s", "ßs");
}

#[test]
fn validation_errors_implement_standard_error_and_have_display_messages() {
    for error in [
        GameError::InvalidWord,
        GameError::InvalidAttempts,
        GameError::InvalidGuess,
        GameError::NotWaitingForInput,
    ] {
        let standard_error: &dyn std::error::Error = &error;
        assert!(!standard_error.to_string().is_empty());
        assert!(standard_error.source().is_none());
    }
}
