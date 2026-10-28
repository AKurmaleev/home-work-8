# Traits and Generics Through Homework 8

Lecture 14 connects polymorphism with Rust's ownership and type system. The two
exercises use that material differently: `MyVec` implements a standard trait for
arbitrary element types, while Hangman models control flow with an enum.

## 1. A generic type does not mean unrestricted operations

```rust
pub struct MyVec<T> {
    data: [Option<T>; 32],
    len: usize,
}
```

`T` can be an integer, a string, a borrowed value, or a custom resource-owning type.
The implementation may move or borrow a `T` without requiring it to be `Copy`,
`Clone`, or `Default`. Calling a capability such as `.clone()` would require a
corresponding bound, so do not add such operations unless the contract needs them.

`std::array::from_fn(|_| None)` constructs the empty slots individually without
requiring an element value to copy. `Default for MyVec<T>` constructs an empty
container; it does not require `T: Default`.

## 2. `Option<T>` expresses occupied and vacant slots

The representation invariant is:

```text
0 <= len <= 32
slots before len: Some(T)
slots from len onward: None
```

Private fields prevent callers from creating a hole inside the occupied prefix.
Methods must preserve that invariant. `len` is not the number of slots allocated:
there are always 32 slots, even when none contains an element.

## 3. `push` and `pop` transfer ownership

```rust
pub fn push(&mut self, value: T) -> Result<(), T>
pub fn pop(&mut self) -> Option<T>
```

A successful push transfers the element into the container. If full, the method
returns `Err(value)`, handing the same element back rather than losing or cloning it.

`pop` decreases the occupied length and uses `Option::take()` to move the last
value out, leaving `None`. The caller now owns the returned element; removing it
does not itself invoke that element's destructor.

## 4. `Iterator::Item` is an associated type

```rust
impl<'a, T> Iterator for MyVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.vec.get(self.pos)?;
        self.pos += 1;
        Some(value)
    }
}
```

`Iterator` describes behavior, while `Item` states what this particular iterator
produces. Here it produces references, not owned elements or clones.

The returned reference is tied to the borrowed container's lifetime `'a`, not
merely to the short borrow of `&mut self` for one call to `next`. Multiple shared
references can therefore remain valid together. A compile-fail doctest verifies
that the container cannot be cleared while a returned reference is still used.

Implementing `Iterator` also enables adapters such as `map`, `filter`, and `collect`
without implementing them separately. Our iterator advances in insertion order
and remains exhausted after reaching the end. `IntoIterator for &MyVec<T>`
returns that same iterator, enabling `for value in &values` and generic borrowing
iteration without consuming the container or adding element bounds.

## 5. Generic parameters and associated types answer different questions

`MyVec<T>` lets its user select the element type. `Iterator::Item` is selected by
the iterator implementation. An associated type expresses one output type for
that implementation; a generic trait parameter can permit separate implementations
for different argument types, subject to coherence.

Bounds should describe capabilities actually required, not a wish list such as
`T: Clone + Default + Debug` added to make implementation convenient.

## 6. Static and dynamic dispatch

Generic code normally supports static dispatch and compiler specialization for
used concrete types. This can enable inlining, but may increase generated code
and compile time. The final machine-code shape depends on optimization.

`&dyn Trait` or `Box<dyn Trait>` allows runtime polymorphism behind an object
boundary. A borrowed trait object does not itself require a heap allocation;
`Box` adds ownership of a heap allocation.

This homework needs no custom trait-object hierarchy: the container implements
`Iterator`, and the game has a closed set of states. Neither dispatch strategy
is universally faster; performance claims require measurements.

A trait method with type parameters or a bare `Self` result can be excluded from
the object interface with `where Self: Sized`. That can preserve the trait's dyn
compatibility while making that method unavailable through `dyn`. This corrects
an inaccurate comment on lecture page 43.

## 7. Cleanup is also part of a generic contract

`clear` calls `pop` before dropping each removed value. The remaining container
already has a valid occupied prefix when caller-controlled `T::drop` runs.

If that destructor panics and unwinds, clearing may stop partway through, but the
container remains usable. The test injects one destructor panic to check this
contract; it does not claim recovery from `panic = "abort"` or repeated panics.

No custom container `Drop` is needed for ordinary destruction: Rust drops the
array fields and their remaining `Some(T)` elements automatically.

## 8. An enum makes the game phase explicit

```text
WaitingForInput -- valid input --> CheckingGuess(letter)
CheckingGuess -- ordinary hit, miss, or repeat --> WaitingForInput
CheckingGuess -- final required letter --> Won
CheckingGuess -- final permitted miss --> Lost
```

Invalid input leaves the waiting state unchanged. Only checking a new absent
letter consumes an attempt. Submitting a guess does not already apply it.

The console loop matches all four states. Waiting performs input, checking
applies exactly one pending letter, and terminal states print the outcome without
reading more input. A pending letter lives inside `CheckingGuess(char)` rather
than in a separate optional field that could disagree with the phase.

The model rejects replacing a pending guess or submitting after termination.
Tests inspect these intermediate states, not just the final text output.

The private `read_input_line` helper owns bounded byte reading, oversized-line
resynchronization, and UTF-8 validation. It returns EOF, a rejected-line message,
or text borrowing the reused input buffer. It does not print or change game state.
`submit_guess` still owns trimming, single-letter validation, and normalization;
empty lines and multiple letters reach that model unchanged. `exit` remains invalid
multi-letter input, not a command. Independent helper tests keep I/O failures
separate from recoverable input rejection, while console tests preserve exact
messages, prompt/final flushing, and pending/terminal-state read behavior.

## 9. Useful boundaries

- `MyVec` and Hangman are independent exercises; a generic container need not be
  forced into unrelated application code.
- Newtypes distinguish domain identities; `Deref` should not be used merely to
  simulate inheritance or silently turn every domain value into a primitive.
- Rust's orphan and coherence rules limit which trait implementations can coexist.
  Adding a broad blanket implementation to a public library can affect downstream users.
- Stable trait-object layout is not a public ABI guarantee; vtable diagrams are
  explanatory models, not a serialization or FFI format.

The practical goal is to express the smallest correct contract: ownership in
signatures, capabilities in trait bounds, element relationships in associated
types, and allowed control-flow phases in an enum.
