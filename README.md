# Homework 8 — Generics, Iterators, and a State Machine

Two independent exercises from the Rust traits and generics assignment:

1. A safe `MyVec<T>` with 32 inline slots and a borrowing iterator.
2. The Homework 7 Hangman game refactored into an explicit finite state machine.

Only the standard library is used. Unsafe code is forbidden.

## Run

Requires Rust 1.85 or later, edition 2024, and a UTF-8 terminal.

```sh
cargo run --locked
```

The game retains the Russian-language interface from Homework 7. Enter one letter
per line. The default secret is `rust`, with five allowed misses; both settings
are constants in [`src/main.rs`](src/main.rs). Repeated or invalid guesses do not
cost attempts. EOF interrupts the session rather than declaring a loss.

`MyVec` is a library exercise, not a replacement for the game's collections:

```rust
use rust_homework_8::MyVec;

let mut words = MyVec::new();
words.push(String::from("hello")).unwrap();
assert_eq!(words.get(0).map(String::as_str), Some("hello"));
assert_eq!(words.pop().as_deref(), Some("hello"));
```

## Reviewer navigation

| File | Contents |
| --- | --- |
| [ASSIGNMENT.md](ASSIGNMENT.md) | Assignment requirements in English |
| [src/my_vec.rs](src/my_vec.rs) | Fixed-capacity container and `MyVecIter` |
| [src/game.rs](src/game.rs) | Game states and explicit transitions |
| [src/console.rs](src/console.rs) | State-driven console loop |
| [src/main.rs](src/main.rs) | Entry point and default game settings |
| [tests/my_vec.rs](tests/my_vec.rs) | Ownership, capacity, iteration, and destruction tests |
| [tests/game.rs](tests/game.rs) | Game rules and transition tests |
| [tests/console.rs](tests/console.rs) | Console scenarios and binary smoke test |
| [REPORT.md](REPORT.md) | Implementation and verification report |
| [LEARNING.md](LEARNING.md) | Explanation of generics, traits, lifetimes, and the FSM |

## Verify

```sh
cargo fmt --all --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --release
```

## Scope and limits

- `MyVec<T>` owns `[Option<T>; 32]`; it never allocates a separate storage buffer.
  Element types such as `String` may allocate internally. No `Copy`, `Clone`,
  `Default`, or `Debug` bound is required for ordinary container operations.
- Full-capacity `push` returns the original element without changing the container.
  `clear` removes elements before dropping them, preserving a valid remaining
  prefix if an element destructor unwinds. It does not promise rollback.
- Hangman uses 1–64 alphabetic Unicode scalar values, not grapheme clusters.
  Single-scalar lowercase mapping is supported; normalization and full case
  folding are not. `е` and `ё` remain distinct letters.
- Console lines are limited to 128 bytes including the terminator. Oversized
  lines and invalid UTF-8 are rejected without a penalty. Input is blocking,
  without a timeout.

## Version

**0.1.0** — both assignment exercises, ownership and transition tests, and documentation.
