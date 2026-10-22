# Homework 8 — Implementation Report

## 1. Fixed-capacity `MyVec<T>`

The container implements the required API over `[Option<T>; 32]` and a private
length. It uses safe Rust and allocates no separate element-storage buffer.
Element types remain free to own heap resources, as in the assignment's `String` tests.

| Requirement | Implementation |
| --- | --- |
| Empty construction | `array::from_fn` creates 32 `None` slots without bounds on `T` |
| Ownership-preserving insertion | `push` stores the element or returns the original `Err(value)` when full |
| Ownership-preserving removal | `pop` decreases length and takes the final occupied slot |
| Checked access | `get` / `get_mut` reject all indices outside the occupied prefix |
| Borrowing iterator | `MyVecIter<'a, T>` implements `Iterator<Item = &'a T>` in insertion order |
| Clear and destruction | `clear` removes before dropping; remaining fields are automatically dropped with the container |

Invariant: `len <= 32`, every slot before `len` is `Some`, and every later slot is
`None`. Ordinary operations require no `Copy`, `Clone`, `Default`, or `Debug` bound.
The optional derived `Debug` implementation is available when the element supports it.

A destructor-unwind test verifies that interrupted `clear` retains a valid
remaining prefix. This is a partial-clear guarantee, not rollback, and does not
cover process abort or a second panic during unwinding.

## 2. Hangman state machine

The Homework 7 game is retained in this independent repository and refactored
around the assignment's four explicit phases:

| State | Processing and transition |
| --- | --- |
| `WaitingForInput` | Validate a line; successful submission stores `CheckingGuess(char)` without applying it |
| `CheckingGuess(char)` | Check the pending letter exactly once; transition to waiting, won, or lost |
| `Won` | Display the final state and word; do not read more input |
| `Lost` | Display the final state and word; do not read more input |

The main console loop is an exhaustive `match game.state()`. Tests observe the
pending state, rejection of pending-guess replacement, and immutable terminal states.

Existing behavior remains: all matching positions open, repeated guesses cost no
attempt, invalid input leaves state unchanged, case matching uses single-scalar
lowercase, UTF-8 errors and oversized lines recover, and EOF interrupts without a
false win or loss. Stream errors propagate and prompts/final output are flushed.
`MyVec` is not forced into the game; the assignment exercises are independent.

## Verification

Native Windows, `x86_64-pc-windows-gnu`, on both Rust **1.85.0** and **1.95.0**:

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed |
| `cargo check --locked --offline --all-targets` | Passed |
| `cargo clippy --locked --offline --all-targets -- -D warnings` | Passed |
| `cargo test --locked --offline` | 81 integration tests and 2 doctests passed |
| `cargo test --locked --offline --release` | The same 81 integration tests and 2 doctests passed |

Test breakdown: **20** container tests, **30** game tests, **31** console tests.
One doctest exercises normal use; the other verifies a borrowing violation fails
to compile. The four assignment-minimum test cases are present. Tests also cover
non-`Copy`/non-`Clone`/non-`Default` elements, capacity and index boundaries,
ownership on overflow/pop, slot reuse, drop counts, and iterator exhaustion.

An independent static review found no actionable defects. Linux and CI execution
were not verified for this submission. No benchmark or performance-gain claim is made.

## Complexity and resource bounds

- `MyVec`: `push`, `pop`, access, iterator creation, and each `next` are `O(1)`;
  iteration and `clear` are `O(n)`, excluding caller-controlled element operations.
  Inline storage is 32 `Option<T>` slots plus length and any layout padding.
- Game: for word length `L` and checked-guess count `G`, a check or mask can take
  `O(L × G)`; a checked new letter uses at most additional linear scans. Word
  length is limited to 64 scalars, with at most `L` distinct hits and the initial
  number of allowed misses. Persistent game storage is `O(L + G)`.
- Console: stores at most 129 input bytes before rejecting an oversized line;
  the remainder is skipped without accumulation. Input remains blocking.
