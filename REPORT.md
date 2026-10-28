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

## Original submission verification

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

The original submission's static review reported no actionable defects. Linux and
CI execution were not verified for that submission. No benchmark or performance-gain
claim is made.

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

## Latest-rules review — 2026-10-28

Reviewed the complete tracked snapshot at `9ba52f3ae32c8f0a8cdefb1df42bdbbb5726588c`,
not only a diff, against `ASSIGNMENT.md`, both original assignment PDFs, and the
current canonical Rust rules. Standards and specification passes were separate.
The safe `[Option<T>; 32]` representation and explicit four-phase game were retained.
No game/FSM behavior defect was found.

Corrections:

- `IntoIterator for &MyVec<T>` returns the existing borrowing iterator. This is an
  additive shared-reference contract: no consuming/mutable iterator, element
  bounds, allocation, storage redesign, or changed `iter()` behavior.
- CLI-test input failure now reports both kill and wait outcomes, preserves the
  initiating error, and attempts wait even after kill fails. A test-only callback
  injects cleanup outcomes without depending on unreliable OS failures.
- README verification enables Rule 35.9's three error-discard lints explicitly.
- Crate documentation tests the game/console workflow alongside the existing
  container examples and compile-fail borrowing example.
- `push_pop_basic` now includes three values, and `iter_borrows` explicitly collects
  `Vec<&String>`, matching the assignment minimum while retaining stronger checks.

Regression evidence:

- After behavior-preserving extraction of CLI-test cleanup, its real-binary smoke
  test passed. Then
  `cargo test --locked --test console cli_cleanup_attempts_both_steps_and_retains_all_failures -- --exact`
  failed because the injected kill error disappeared. The identical test passed
  after aggregation was fixed and checks all four kill/wait outcome combinations.
- `cargo test --locked --test my_vec borrowed_into_iterator_preserves_order_references_and_ownership_without_element_bounds -- --exact`
  failed with `E0277` because `&MyVec<T>` lacked `IntoIterator`, then passed after
  the trait implementation. A later lint-only lifetime elision kept its assertions
  unchanged. Coverage checks empty iteration, reference identity, order, no element
  trait bounds, and retained ownership.
- The initial strict Clippy gate failed on two discarded CLI cleanup results.
  A new test helper later triggered `needless_lifetimes`; eliding that lifetime
  resolved it without an allowance or weakened assertion.

Final validation on Rust **1.99.1**, native `x86_64-pc-windows-gnu`:

- `cargo fmt --all --check` — passed.
- `cargo check --locked --all-targets` — passed.
- `cargo clippy --locked --all-targets -- -D warnings -D unused_must_use -D clippy::let_underscore_must_use -D clippy::unused_result_ok` — passed.
- `cargo test --locked` and `cargo test --locked --release` — each passed
  **83 integration tests** (21 container, 30 game, 32 console) and **3 doctests**.
- `cargo doc --locked --no-deps` — passed.

Rust **1.85.0**, explicitly selected as `+1.85.0-x86_64-pc-windows-gnu`, also
passed `check --locked --all-targets`, `test --locked`, and
`test --locked --release` with those same test counts.
Some current-toolchain builds emitted a nonfatal incremental-cache finalization
note (`Access is denied`, OS error 5). Checks still completed successfully.
Linux, MSVC linking, and remote CI were not verified. CLI smoke tests still rely
on the caller's bounded test-command deadline; they have no internal process
timeout. No benchmarks or performance improvements are claimed.

## Instructor-feedback delta — input boundary extraction

HW8 inherited the same inline bounded-read/UTF-8 block as HW7, so its waiting
branch now calls the same private helper interface:
`read_input_line<'a>(&mut impl BufRead, &'a mut Vec<u8>) -> io::Result<InputLine<'a>>`.
A private outcome enum separates EOF, rejected-line messages, and text borrowing
the reused buffer. The helper performs bounded reading, oversized-line
resynchronization, and UTF-8 validation without printing or changing game state.
The model retains letter validation/normalization and pending-guess transitions;
all public contracts and exact console output remain unchanged. `exit` remains
invalid multi-letter input, not a command. `MyVec` was not changed by this delta.

This was a behavior-preserving refactor. Before replacing either loop block,
seven direct tests of a test-scoped helper copy and a new exact-transcript matrix
passed alongside the original public console path.
`cargo test --locked --lib --test console --test game` passed before and after
wiring the same helper into production: **7 unit + 33 console + 30 game** tests.
Assertions were unchanged across the extraction. No bug or runtime-red evidence
is claimed for this refactor.

Helper cases cover unmodified Unicode/CRLF/unterminated text, empty lines versus
EOF, buffer reuse, 128/129-byte limits, invalid UTF-8, oversized-line discard and
next-line preservation, one-byte buffering, and stream failures before input,
after partial input, and during discard. The transcript matrix checks six rejection
inputs followed by either Unicode victory or EOF. Existing tests still verify
that pending guesses are checked before further reads and terminal games do not read.

Superseding final counts: **7 unit tests, 84 integration tests** (21 container,
30 game, 33 console), and **3 doctests**. All of these commands passed on the final source:

```sh
cargo fmt --all --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings -D unused_must_use -D clippy::let_underscore_must_use -D clippy::unused_result_ok
cargo test --locked
cargo test --locked --release
cargo doc --locked --no-deps
cargo +1.85.0-x86_64-pc-windows-gnu check --locked --all-targets
cargo +1.85.0-x86_64-pc-windows-gnu test --locked
cargo +1.85.0-x86_64-pc-windows-gnu test --locked --release
```

Current-toolchain tests used Rust 1.99.1; MSRV tests used Rust 1.85.0. Debug and
release runs passed those superseding counts. Prior platform/CI caveats still
apply. No dependency, public API, shared-rule, staging, commit, or push change.
