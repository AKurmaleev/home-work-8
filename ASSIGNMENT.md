# Homework 8 — Assignment

English transcription of `ДЗ RUST-2_.pdf`.

## 1. Implement `MyVec<T>` without unsafe code

Implement a simplified vector over a fixed-size array, without a heap allocator
for the container's storage and without unsafe code. The exercise practices
ownership, borrowing, and lifetimes.

Required representation:

```rust
pub struct MyVec<T> {
    data: [Option<T>; 32],
    len: usize,
}
```

Required methods:

| Method | Signature / result |
| --- | --- |
| `new` | `new() -> Self` |
| `push` | `push(&mut self, value: T) -> Result<(), T>`; error when full |
| `pop` | `pop(&mut self) -> Option<T>` |
| `len` | `len(&self) -> usize` |
| `is_empty` | `is_empty(&self) -> bool` |
| `get` | `get(&self, idx: usize) -> Option<&T>` |
| `get_mut` | `get_mut(&mut self, idx: usize) -> Option<&mut T>` |
| `iter` | `iter(&self) -> MyVecIter<'_, T>` |
| `clear` | `clear(&mut self)` |

The iterator stores a borrowed `&'a MyVec<T>` and its current position. Implement
`Iterator` with `type Item = &'a T` and `next(&mut self) -> Option<Self::Item>`.

Minimum tests:

- `push_pop_basic`: push three values, pop in reverse order, and check length.
- `iter_borrows`: iterate over stored `String` values as `&String`.
- `capacity_overflow`: fill all 32 slots and verify another push fails.
- `drop_called`: use an `Rc<Cell<i32>>` counter to verify elements are dropped by `clear`.

## 2. Refactor the previous game into a finite state machine

Rework the existing game's logic around an enum representing its states:

```rust
enum GameState {
    WaitingForInput,
    CheckingGuess(char),
    Won,
    Lost,
}
```

A different state structure is permitted if it fits the implementation better.

Transitions must be explicit:

- `WaitingForInput`: wait for user input.
- `CheckingGuess`: check the submitted letter.
- `Won`: every letter in the word has been guessed.
- `Lost`: no attempts remain.

The main game loop must handle the current state through `match`, rather than
being a collection of unrelated `if`/`else` branches.

Tests are mandatory. Existing game behavior is retained from Homework 7.
