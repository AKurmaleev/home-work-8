use std::cell::Cell;
use std::rc::Rc;

use rust_homework_8::{MyVec, MyVecIter};

struct NoTraits(String);

struct DropProbe {
    id: usize,
    drops: Rc<Cell<usize>>,
}

impl DropProbe {
    fn new(id: usize, drops: &Rc<Cell<usize>>) -> Self {
        Self {
            id,
            drops: Rc::clone(drops),
        }
    }
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

fn borrowed_iter<T>(values: &MyVec<T>) -> MyVecIter<'_, T> {
    values.iter()
}

fn first_borrowed<T>(mut iter: MyVecIter<'_, T>) -> Option<&T> {
    iter.next()
}

#[test]
fn new_is_empty_and_empty_operations_are_repeatable() {
    let mut values = MyVec::<NoTraits>::new();

    for _ in 0..3 {
        assert_eq!(values.len(), 0);
        assert!(values.is_empty());
        assert!(values.get(0).is_none());
        assert!(values.get_mut(0).is_none());
        assert!(values.pop().is_none());
        assert!(values.iter().next().is_none());
        values.clear();
    }
}

#[test]
fn push_pop_basic() {
    let mut values = MyVec::new();

    assert_eq!(values.push(10), Ok(()));
    assert_eq!(values.len(), 1);
    assert!(!values.is_empty());
    assert_eq!(values.push(20), Ok(()));
    assert_eq!(values.len(), 2);
    assert_eq!(values.get(0), Some(&10));
    assert_eq!(values.get(1), Some(&20));
    assert_eq!(values.push(30), Ok(()));
    assert_eq!(values.len(), 3);
    assert_eq!(values.pop(), Some(30));
    assert_eq!(values.len(), 2);
    assert_eq!(values.pop(), Some(20));
    assert_eq!(values.len(), 1);
    assert_eq!(values.get(1), None);
    assert_eq!(values.pop(), Some(10));
    assert_eq!(values.len(), 0);
    assert!(values.is_empty());
    assert_eq!(values.pop(), None);
}

#[test]
fn iter_borrows() {
    let mut strings = MyVec::<String>::new();
    strings.push(String::from("a")).unwrap();
    strings.push(String::from("b")).unwrap();
    let collected: Vec<&String> = strings.iter().collect();
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0], "a");
    assert_eq!(collected[1], "b");

    let mut values = MyVec::new();
    assert!(values.push(NoTraits(String::from("first"))).is_ok());
    assert!(values.push(NoTraits(String::from("second"))).is_ok());

    let mut iter: MyVecIter<'_, NoTraits> = borrowed_iter(&values);
    let first: &NoTraits = iter.next().expect("first borrowed item");
    let second: &NoTraits = iter.next().expect("second borrowed item");
    assert_eq!(first.0, "first");
    assert_eq!(second.0, "second");
    assert!(std::ptr::eq(first, values.get(0).unwrap()));
    assert!(std::ptr::eq(second, values.get(1).unwrap()));
    assert!(iter.next().is_none());
    assert_eq!(first.0, "first");
    assert_eq!(values.len(), 2);

    let first = first_borrowed(values.iter()).expect("reference outlives iterator");
    assert_eq!(first.0, "first");
    assert_eq!(values.pop().unwrap().0, "second");
    assert_eq!(values.pop().unwrap().0, "first");
}

#[test]
fn capacity_overflow() {
    let mut values = MyVec::new();

    for value in 0..32 {
        assert_eq!(values.push(value), Ok(()));
        assert_eq!(values.len(), value + 1);
    }
    assert_eq!(values.get(31), Some(&31));
    for rejected in [32, 99, usize::MAX] {
        assert_eq!(values.push(rejected), Err(rejected));
        assert_eq!(values.len(), 32);
        assert!(!values.is_empty());
        assert!(values.iter().copied().eq(0..32));
    }
    for expected in (0..32).rev() {
        assert_eq!(values.pop(), Some(expected));
        assert_eq!(values.len(), expected);
    }
    assert!(values.is_empty());
    assert_eq!(values.pop(), None);
}

#[test]
fn overflow_returns_original_rc_and_preserves_every_stored_value() {
    let mut values = MyVec::new();
    let stored: [Rc<Cell<usize>>; 32] = std::array::from_fn(|id| Rc::new(Cell::new(id)));
    for value in &stored {
        assert!(values.push(Rc::clone(value)).is_ok());
    }

    let original = Rc::new(Cell::new(100));
    let returned = match values.push(Rc::clone(&original)) {
        Err(value) => value,
        Ok(()) => panic!("the thirty-third value must be rejected"),
    };
    assert!(Rc::ptr_eq(&returned, &original));
    assert_eq!(Rc::strong_count(&original), 2);
    assert_eq!(values.len(), 32);
    assert_eq!(values.iter().count(), 32);
    for (index, expected) in stored.iter().enumerate() {
        let actual = values.get(index).expect("stored item remains present");
        assert!(Rc::ptr_eq(actual, expected));
        assert_eq!(actual.get(), index);
        assert_eq!(Rc::strong_count(expected), 2);
    }
    drop(returned);
    assert_eq!(Rc::strong_count(&original), 1);
    drop(values);
    for value in &stored {
        assert_eq!(Rc::strong_count(value), 1);
    }
}

#[test]
fn checked_access_rejects_len_capacity_and_usize_max() {
    let mut values = MyVec::new();

    for length in 0..=32 {
        assert_eq!(values.len(), length);
        for index in [length, 32, 33, usize::MAX] {
            assert!(values.get(index).is_none(), "len={length}, index={index}");
            assert!(
                values.get_mut(index).is_none(),
                "len={length}, index={index}"
            );
        }
        for index in 0..length {
            assert_eq!(values.get(index), Some(&index));
        }
        if length < 32 {
            assert_eq!(values.push(length), Ok(()));
        }
    }
    assert_eq!(values.len(), 32);
    assert!(values.iter().copied().eq(0..32));
}

#[test]
fn get_mut_updates_first_middle_and_last_without_changing_length() {
    let mut values = MyVec::new();
    for value in 0..32 {
        assert_eq!(values.push(value), Ok(()));
    }

    for (index, replacement) in [(0, 100), (15, 200), (31, 300)] {
        *values.get_mut(index).expect("in-bounds mutable item") = replacement;
        assert_eq!(values.get(index), Some(&replacement));
        assert_eq!(values.len(), 32);
    }
    assert_eq!(values.get(1), Some(&1));
    assert_eq!(values.get(30), Some(&30));
    assert_eq!(values.pop(), Some(300));
    assert!(values.get_mut(31).is_none());
    assert_eq!(values.len(), 31);
}

#[test]
fn all_operations_accept_non_copy_non_clone_non_default_non_debug_elements() {
    let mut values = MyVec::<NoTraits>::new();
    for id in 0..32 {
        assert!(values.push(NoTraits(id.to_string())).is_ok());
    }
    let rejected = match values.push(NoTraits(String::from("rejected"))) {
        Err(value) => value,
        Ok(()) => panic!("full vector must return ownership"),
    };
    assert_eq!(rejected.0, "rejected");
    assert_eq!(values.get(0).unwrap().0, "0");
    values.get_mut(0).unwrap().0.push_str(" changed");
    assert_eq!(
        first_borrowed(borrowed_iter(&values)).unwrap().0,
        "0 changed"
    );
    assert_eq!(values.pop().unwrap().0, "31");
    assert_eq!(values.len(), 31);
    values.clear();
    assert!(values.is_empty());
    assert!(values.push(rejected).is_ok());
    assert_eq!(values.pop().unwrap().0, "rejected");
}

#[test]
fn borrowed_into_iterator_preserves_order_references_and_ownership_without_element_bounds() {
    fn borrowed<T>(values: &MyVec<T>) -> impl Iterator<Item = &T> {
        IntoIterator::into_iter(values)
    }

    let mut values = MyVec::<NoTraits>::new();
    assert!(borrowed(&values).next().is_none());
    assert!(values.push(NoTraits(String::from("first"))).is_ok());
    assert!(values.push(NoTraits(String::from("second"))).is_ok());

    let references: Vec<&NoTraits> = borrowed(&values).collect();
    assert_eq!(references.len(), 2);
    assert!(std::ptr::eq(references[0], values.get(0).unwrap()));
    assert!(std::ptr::eq(references[1], values.get(1).unwrap()));
    let mut texts = Vec::new();
    for value in &values {
        texts.push(value.0.as_str());
    }
    assert_eq!(texts, ["first", "second"]);
    assert_eq!(values.pop().unwrap().0, "second");
    assert_eq!(values.pop().unwrap().0, "first");
}

#[test]
fn default_does_not_require_element_traits() {
    let mut values: MyVec<NoTraits> = Default::default();

    assert_eq!(values.len(), 0);
    assert!(values.is_empty());
    assert!(values.iter().next().is_none());
    assert!(values.push(NoTraits(String::from("owned"))).is_ok());
    assert_eq!(values.pop().unwrap().0, "owned");
    assert!(values.is_empty());
}

#[test]
fn pop_transfers_ownership_without_early_drop() {
    let drops = Rc::new(Cell::new(0));
    let mut values = MyVec::new();
    assert!(values.push(DropProbe::new(7, &drops)).is_ok());

    let popped = values.pop().expect("owned item");
    assert_eq!(popped.id, 7);
    assert_eq!(drops.get(), 0);
    assert!(values.is_empty());
    assert!(values.get(0).is_none());
    drop(values);
    assert_eq!(drops.get(), 0);
    drop(popped);
    assert_eq!(drops.get(), 1);
}

#[test]
fn drop_called() {
    struct Counted(Rc<Cell<i32>>);

    impl Drop for Counted {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    let drops = Rc::new(Cell::new(0));
    let mut values = MyVec::new();
    for _ in 0..32 {
        assert!(values.push(Counted(Rc::clone(&drops))).is_ok());
    }
    assert_eq!(drops.get(), 0);
    values.clear();
    assert_eq!(drops.get(), 32);
    assert!(values.is_empty());
    drop(values);
    assert_eq!(drops.get(), 32);
}

#[test]
fn partial_pop_leaves_removed_values_to_the_caller_only() {
    let counters: [Rc<Cell<usize>>; 4] = std::array::from_fn(|_| Rc::new(Cell::new(0)));
    let mut values = MyVec::new();
    for (id, counter) in counters.iter().enumerate() {
        assert!(values.push(DropProbe::new(id, counter)).is_ok());
    }

    let last = values.pop().unwrap();
    let previous = values.pop().unwrap();
    assert_eq!((last.id, previous.id), (3, 2));
    assert_eq!(values.len(), 2);
    assert!(values.iter().map(|value| value.id).eq([0, 1]));
    assert!(counters.iter().all(|counter| counter.get() == 0));

    drop(values);
    assert_eq!(
        counters.each_ref().map(|counter| counter.get()),
        [1, 1, 0, 0]
    );
    drop(previous);
    assert_eq!(
        counters.each_ref().map(|counter| counter.get()),
        [1, 1, 1, 0]
    );
    drop(last);
    assert_eq!(
        counters.each_ref().map(|counter| counter.get()),
        [1, 1, 1, 1]
    );
}

#[test]
fn clear_twice_drops_each_value_once_and_allows_full_reuse() {
    let drops = Rc::new(Cell::new(0));
    let mut values = MyVec::new();
    for id in 0..32 {
        assert!(values.push(DropProbe::new(id, &drops)).is_ok());
    }

    values.clear();
    assert_eq!(drops.get(), 32);
    assert_eq!(values.len(), 0);
    assert!(values.is_empty());
    assert!(values.get(0).is_none());
    assert!(values.get_mut(31).is_none());
    assert!(values.pop().is_none());
    assert!(values.iter().next().is_none());
    values.clear();
    assert_eq!(drops.get(), 32);

    for id in 32..64 {
        assert!(values.push(DropProbe::new(id, &drops)).is_ok());
    }
    assert_eq!(values.len(), 32);
    assert!(values.iter().map(|value| value.id).eq(32..64));
    assert_eq!(drops.get(), 32);
    drop(values);
    assert_eq!(drops.get(), 64);
}

#[test]
fn popped_slots_can_be_reused_without_stale_values_or_extra_drops() {
    let counters: [Rc<Cell<usize>>; 3] = std::array::from_fn(|_| Rc::new(Cell::new(0)));
    let mut values = MyVec::new();
    assert!(values.push(DropProbe::new(0, &counters[0])).is_ok());
    assert!(values.push(DropProbe::new(1, &counters[1])).is_ok());
    let removed = values.pop().unwrap();
    assert!(values.push(DropProbe::new(2, &counters[2])).is_ok());

    assert_eq!(values.len(), 2);
    assert_eq!(values.get(1).unwrap().id, 2);
    assert!(values.iter().map(|value| value.id).eq([0, 2]));
    assert!(counters.iter().all(|counter| counter.get() == 0));
    values.clear();
    assert_eq!(counters.each_ref().map(|counter| counter.get()), [1, 0, 1]);
    drop(removed);
    drop(values);
    assert_eq!(counters.each_ref().map(|counter| counter.get()), [1, 1, 1]);
}

#[test]
fn shared_references_remain_valid_across_read_only_access_and_iteration() {
    let mut values = MyVec::new();
    assert!(values.push(String::from("alpha")).is_ok());
    assert!(values.push(String::from("beta")).is_ok());
    let first = values.get(0).unwrap();
    let second = values.get(1).unwrap();

    for _ in 0..3 {
        let mut iter = values.iter();
        assert!(std::ptr::eq(first, iter.next().unwrap()));
        assert!(std::ptr::eq(second, iter.next().unwrap()));
        assert!(iter.next().is_none());
        assert_eq!(values.len(), 2);
        assert_eq!(first, "alpha");
        assert_eq!(second, "beta");
    }
}

#[test]
fn iteration_order_and_size_hint_are_exact_until_and_after_exhaustion() {
    let mut values = MyVec::new();
    for value in 0..32 {
        assert_eq!(values.push(value), Ok(()));
    }
    assert_eq!(values.pop(), Some(31));
    assert_eq!(values.push(99), Ok(()));

    let mut iter = values.iter();
    for expected in 0..31 {
        let remaining = 32 - expected;
        assert_eq!(iter.size_hint(), (remaining, Some(remaining)));
        assert_eq!(iter.next(), Some(&expected));
    }
    assert_eq!(iter.size_hint(), (1, Some(1)));
    assert_eq!(iter.next(), Some(&99));
    for _ in 0..5 {
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.next(), None);
    }
    assert_eq!(values.len(), 32);
}

#[test]
fn empty_iteration_is_repeatedly_exhausted_with_an_exact_size_hint() {
    let values = MyVec::<NoTraits>::new();
    let mut iter: MyVecIter<'_, NoTraits> = values.iter();

    for _ in 0..5 {
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert!(iter.next().is_none());
    }
    assert!(values.is_empty());
}

#[test]
fn zero_sized_elements_still_have_capacity_32_and_can_reuse_the_last_slot() {
    let mut values = MyVec::new();
    for _ in 0..32 {
        assert_eq!(values.push(()), Ok(()));
    }
    assert_eq!(values.len(), 32);
    assert_eq!(values.push(()), Err(()));
    assert_eq!(values.iter().count(), 32);
    assert_eq!(values.pop(), Some(()));
    assert_eq!(values.len(), 31);
    assert_eq!(values.push(()), Ok(()));
    assert_eq!(values.len(), 32);
    values.clear();
    assert!(values.is_empty());
}

#[test]
fn elements_may_borrow_non_static_data() {
    let first = String::from("first");
    let second = String::from("second");
    let mut values = MyVec::new();
    assert_eq!(values.push(first.as_str()), Ok(()));
    assert_eq!(values.push(second.as_str()), Ok(()));
    assert!(values.iter().copied().eq(["first", "second"]));
    assert_eq!(values.pop(), Some("second"));
    values.clear();
    assert!(values.is_empty());
    assert_eq!(first, "first");
    assert_eq!(second, "second");
}

#[cfg(panic = "unwind")]
#[test]
fn clear_removes_before_drop_and_remains_usable_after_one_destructor_panics() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    struct PanicOnce {
        id: usize,
        drops: Rc<[Cell<usize>; 6]>,
        panics: Rc<Cell<usize>>,
    }

    impl Drop for PanicOnce {
        fn drop(&mut self) {
            self.drops[self.id].set(self.drops[self.id].get() + 1);
            if self.id == 2 && self.panics.get() == 0 {
                self.panics.set(1);
                panic!("intentional destructor panic");
            }
        }
    }

    let drops = Rc::new(std::array::from_fn(|_| Cell::new(0)));
    let panics = Rc::new(Cell::new(0));
    let make_value = |id| PanicOnce {
        id,
        drops: Rc::clone(&drops),
        panics: Rc::clone(&panics),
    };
    let mut values = MyVec::new();
    for id in 0..4 {
        assert!(values.push(make_value(id)).is_ok());
    }

    // Unwinding is confined to this test; validate the surviving prefix before reuse.
    let result = catch_unwind(AssertUnwindSafe(|| values.clear()));
    assert!(result.is_err());
    assert_eq!(panics.get(), 1);
    assert_eq!(drops.each_ref().map(Cell::get), [0, 0, 1, 1, 0, 0]);
    assert_eq!(values.len(), 2);
    assert!(!values.is_empty());
    assert_eq!(values.get(0).unwrap().id, 0);
    assert_eq!(values.get(1).unwrap().id, 1);
    assert!(values.get(2).is_none());
    assert!(values.get_mut(2).is_none());
    assert!(values.iter().map(|value| value.id).eq([0, 1]));

    assert!(values.push(make_value(4)).is_ok());
    assert_eq!(values.len(), 3);
    let popped = values.pop().expect("reused slot transfers ownership");
    assert_eq!(popped.id, 4);
    assert_eq!(drops[4].get(), 0);
    assert_eq!(values.len(), 2);
    values.clear();
    values.clear();
    assert!(values.is_empty());
    assert!(values.pop().is_none());
    assert!(values.iter().next().is_none());
    assert_eq!(drops.each_ref().map(Cell::get), [1, 1, 1, 1, 0, 0]);

    assert!(values.push(make_value(5)).is_ok());
    drop(values);
    assert_eq!(drops.each_ref().map(Cell::get), [1, 1, 1, 1, 0, 1]);
    drop(popped);
    assert_eq!(drops.each_ref().map(Cell::get), [1, 1, 1, 1, 1, 1]);
    assert_eq!(panics.get(), 1);
}
