#![cfg(test)]
#![allow(clippy::disallowed_names)]

// Regression test for
// https://github.com/tim-harding/soa-rs/issues/17
#[allow(dead_code)]
#[derive(Soars)]
struct AllowUnknownAttributes;

use soa_rs::{AsMutSlice, AsSlice, AsSoaRef, Soa, SoaClone, Soars, soa};

#[derive(Soars, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[soa_derive(Debug, PartialEq, Eq)]
struct Foo(u8);
use std::sync::Mutex;

#[derive(Soars, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[soa_derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ExtraImplTester {
    things: u8,
    stuff: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SingleDrop(u8);

impl SingleDrop {
    pub const DEFAULT: Self = Self(0);
}

impl Drop for SingleDrop {
    fn drop(&mut self) {
        assert_eq!(self.0, 0);
        self.0 += 1;
    }
}

#[derive(Soars, Debug, Clone, PartialEq, Eq, Hash)]
#[soa_array]
#[soa_derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct El {
    foo: u64,
    bar: u8,
    baz: SingleDrop,
}

const A: El = El {
    foo: 0,
    bar: 1,
    baz: SingleDrop::DEFAULT,
};

const B: El = El {
    foo: 4,
    bar: 5,
    baz: SingleDrop::DEFAULT,
};

const C: El = El {
    foo: 8,
    bar: 9,
    baz: SingleDrop::DEFAULT,
};

const D: El = El {
    foo: 12,
    bar: 13,
    baz: SingleDrop::DEFAULT,
};

const E: El = El {
    foo: 16,
    bar: 17,
    baz: SingleDrop::DEFAULT,
};

const ABCDE: [El; 5] = [A, B, C, D, E];
const ABCDE_SOA: ElArray<5> = ElArray::from_array([A, B, C, D, E]);

#[derive(Soars, Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
#[soa_derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Unit;

#[derive(Soars, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[soa_derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Empty {}

#[derive(Soars, SoaClone, Clone, Copy, PartialEq, Eq, Default, Debug)]
#[soa_derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct EmptyTuple();

#[derive(Soars, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[soa_derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ZstFields {
    a: Unit,
    b: (),
}

#[derive(Soars, SoaClone, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[soa_derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Tuple(u8, u16, u32);

#[derive(Soars, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[soa_derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Generics<A, B> {
    a: A,
    b: B,
    c: i32,
}

#[test]
pub fn generics() {
    let mut soa = Soa::<Generics<u8, f64>>::new();
    let elements = [
        Generics { a: 1, b: 2.0, c: 3 },
        Generics { a: 4, b: 5.0, c: 6 },
        Generics { a: 7, b: 8.0, c: 9 },
    ];
    for element in elements {
        soa.push(element);
    }
    assert!(elements.into_iter().eq(soa.into_iter()));
}

#[test]
pub fn tuple() {
    let mut soa = Soa::<Tuple>::new();
    let elements = [Tuple(0, 1, 2), Tuple(3, 4, 5), Tuple(6, 7, 8)];
    for element in elements {
        soa.push(element);
    }
    assert!(elements.into_iter().eq(soa.into_iter()));
}

#[test]
pub fn zst_fields() {
    let mut soa = Soa::<ZstFields>::new();
    for _ in 0..5 {
        soa.push(ZstFields::default());
    }
    for _ in 0..5 {
        assert_eq!(soa.pop(), Some(ZstFields::default()));
    }
    assert_eq!(soa.pop(), None);
}

#[test]
pub fn empty_tuple() {
    let mut soa = Soa::<EmptyTuple>::new();
    for _ in 0..5 {
        soa.push(EmptyTuple());
    }
    for _ in 0..5 {
        assert_eq!(soa.pop(), Some(EmptyTuple()));
    }
    assert_eq!(soa.pop(), None);
}

#[test]
pub fn empty_struct() {
    let mut soa = Soa::<Empty>::new();
    for _ in 0..5 {
        soa.push(Empty {});
    }
    for _ in 0..5 {
        assert_eq!(soa.pop(), Some(Empty {}));
    }
    assert_eq!(soa.pop(), None);
}

#[test]
pub fn unit_struct() {
    let mut soa = Soa::<Unit>::new();
    for _ in 0..5 {
        soa.push(Unit);
    }
    for _ in 0..5 {
        assert_eq!(soa.pop(), Some(Unit));
    }
    assert_eq!(soa.pop(), None);
}

#[test]
pub fn push_and_pop() {
    let mut soa = Soa::<El>::new();
    for element in ABCDE.into_iter() {
        soa.push(element);
    }
    for element in ABCDE.into_iter().rev() {
        assert_eq!(Some(element), soa.pop());
    }
}

#[test]
pub fn insert() {
    test_insert(0, [B, A, A, A]);
    test_insert(1, [A, B, A, A]);
    test_insert(2, [A, A, B, A]);
    test_insert(3, [A, A, A, B]);
}

fn test_insert(index: usize, expected: [El; 4]) {
    let mut soa = Soa::<El>::new();
    for element in [A, A, A].into_iter() {
        soa.push(element);
    }
    soa.insert(index, B);
    assert!(soa.into_iter().eq(expected.into_iter()));
}

#[test]
pub fn remove() {
    test_remove(0, A, [B, C, D, E]);
    test_remove(1, B, [A, C, D, E]);
    test_remove(2, C, [A, B, D, E]);
    test_remove(3, D, [A, B, C, E]);
    test_remove(4, E, [A, B, C, D]);
}

fn test_remove(index: usize, expected_return: El, expected_contents: [El; 4]) {
    let mut soa = Soa::<El>::new();
    for element in ABCDE.into_iter() {
        soa.push(element);
    }
    assert_eq!(expected_return, soa.remove(index));
    assert!(soa.into_iter().eq(expected_contents.into_iter()));
}

#[test]
pub fn with_capacity() {
    let mut soa = Soa::<El>::with_capacity(5);
    assert_eq!(soa.capacity(), 5);
    assert_eq!(soa.len(), 0);
    for element in ABCDE.into_iter() {
        soa.push(element);
    }
    assert_eq!(soa.capacity(), 5);
    assert_eq!(soa.len(), 5);
}

#[test]
pub fn from_iter() {
    let soa: Soa<_> = ABCDE.into_iter().collect();
    assert!(soa.into_iter().eq(ABCDE.into_iter()));
}

#[test]
pub fn iter() {
    let soa: Soa<_> = ABCDE.into();
    for (borrowed, owned) in soa.iter().zip(ABCDE.into_iter()) {
        assert_eq!(borrowed.foo, &owned.foo);
        assert_eq!(borrowed.bar, &owned.bar);
        assert_eq!(borrowed.baz, &owned.baz);
    }
}

#[test]
pub fn iter_mut() {
    let mut soa: Soa<_> = ABCDE.into();
    for el in soa.iter_mut() {
        *el.foo += 1;
        *el.bar += 2;
    }
    for (borrowed, owned) in soa.iter().zip(ABCDE.into_iter()) {
        assert_eq!(borrowed.foo, &(owned.foo + 1));
        assert_eq!(borrowed.bar, &(owned.bar + 2));
    }
}

#[test]
pub fn from_impls() {
    let expected: Soa<_> = ABCDE.into_iter().collect();
    let array: [El; 5] = ABCDE;
    let array_ref: &[El; 5] = &ABCDE;
    let mut tmp = ABCDE;
    let array_ref_mut: &mut [El; 5] = &mut tmp;
    assert_eq!(expected, Soa::from(array));
    assert_eq!(expected, Soa::from(array_ref));
    assert_eq!(expected, Soa::from(array_ref_mut));
}

#[test]
pub fn extend() {
    let mut soa: Soa<_> = [A, B].into();
    soa.extend([C, D]);
    assert!(soa.into_iter().eq([A, B, C, D].into_iter()));
}

#[test]
pub fn clone() {
    let expected: Soa<_> = [Tuple(1, 2, 3), Tuple(4, 5, 6), Tuple(7, 8, 9)].into();
    let actual = expected.clone();
    assert_eq!(expected, actual);
}

#[test]
pub fn clone_from() {
    let mut dst: Soa<_> = std::iter::repeat_n(Tuple(100, 100, 100), 7).collect();
    let src: Soa<_> = [Tuple(1, 2, 3), Tuple(4, 5, 6), Tuple(7, 8, 9)].into();
    dst.clone_from(&src);
    assert_eq!(dst, src);
}

#[test]
pub fn partial_ordering_and_equality() {
    #[derive(Soars, Debug, PartialEq, PartialOrd, Clone, Copy)]
    #[soa_derive(Debug, PartialEq, PartialOrd)]
    struct A(f32);

    let cases = [
        (&[][..], &[][..]),
        (&[A(1.), A(2.), A(3.)][..], &[A(1.), A(2.), A(3.)][..]),
        (&[A(1.), A(2.), A(2.)][..], &[A(1.), A(2.), A(3.)][..]),
        (&[A(1.), A(2.), A(4.)][..], &[A(1.), A(2.), A(3.)][..]),
        (
            &[A(1.), A(2.), A(3.)][..],
            &[A(1.), A(2.), A(3.), A(0.)][..],
        ),
        (
            &[A(1.), A(2.), A(3.), A(0.)][..],
            &[A(1.), A(2.), A(3.)][..],
        ),
        (&[A(1.)][..], &[A(f32::NAN)][..]),
    ];

    for case in cases {
        let (l, r) = case;
        let expected_cmp = l.partial_cmp(r);
        let expected_eq = l == r;
        let l: Soa<_> = l.into();
        let r: Soa<_> = r.into();
        assert_eq!(l.partial_cmp(&r), expected_cmp);
        assert_eq!(l == r, expected_eq);
    }
}

#[test]
pub fn ordering() {
    #[derive(Soars, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    #[soa_derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct A(u8);

    let cases = [
        (&[][..], &[][..]),
        (&[A(1), A(2), A(3)][..], &[A(1), A(2), A(3)][..]),
        (&[A(1), A(2), A(2)][..], &[A(1), A(2), A(3)][..]),
        (&[A(1), A(2), A(4)][..], &[A(1), A(2), A(3)][..]),
        (&[A(1), A(2), A(3)][..], &[A(1), A(2), A(3), A(0)][..]),
        (&[A(1), A(2), A(3), A(0)][..], &[A(1), A(2), A(3)][..]),
    ];

    for case in cases {
        let (l, r) = case;
        let expected = l.cmp(r);
        let l: Soa<_> = l.into();
        let r: Soa<_> = r.into();
        let actual = l.cmp(&r);
        assert_eq!(actual, expected);
    }
}

#[test]
pub fn hashing() {
    use core::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut expected = DefaultHasher::new();
    ABCDE.hash(&mut expected);

    let mut actual = DefaultHasher::new();
    let soa: Soa<_> = ABCDE.into();
    soa.hash(&mut actual);

    assert_eq!(actual.finish(), expected.finish());
}

#[test]
pub fn get_index() {
    let soa: Soa<_> = ABCDE.into();
    let actual = soa.get(2).unwrap();
    let actual = El {
        foo: *actual.foo,
        bar: *actual.bar,
        baz: Default::default(),
    };
    assert_eq!(actual, ABCDE[2]);
}

#[test]
pub fn swap() {
    let mut soa: Soa<_> = [A, B, C].into();
    soa.swap(0, 2);
    assert!([C, B, A].into_iter().eq(soa.into_iter()));
}

#[test]
pub fn macro_no_elements() {
    let a: Soa<El> = Soa::new();
    let b = soa![];
    assert_eq!(a, b);
}

#[test]
pub fn field_getters() {
    let mut soa: Soa<_> = ABCDE.into();

    assert_eq!(soa.foo(), &[0, 4, 8, 12, 16]);
    assert_eq!(soa.bar(), &[1, 5, 9, 13, 17]);

    for el in soa.foo_mut() {
        *el += 1;
    }

    for el in soa.bar_mut() {
        *el += 1;
    }

    assert_eq!(soa.foo(), &[1, 5, 9, 13, 17]);
    assert_eq!(soa.bar(), &[2, 6, 10, 14, 18]);
}

#[derive(Debug, Clone, Copy, PartialEq, Soars)]
#[soa_array]
#[soa_derive(Debug, PartialEq, PartialOrd)]
struct Alignment {
    #[align(64)]
    a: f32,
    #[align(64)]
    b: f32,
    #[align(64)]
    c: f32,
    #[align(64)]
    d: f32,
}

#[test]
pub fn align_attribute() {
    let aligns = [
        Alignment {
            a: 0.0,
            b: 1.0,
            c: 2.0,
            d: 3.0,
        },
        Alignment {
            a: 4.0,
            b: 5.0,
            c: 6.0,
            d: 7.0,
        },
        Alignment {
            a: 8.0,
            b: 9.0,
            c: 10.0,
            d: 11.0,
        },
    ];
    let aligns_array = AlignmentArray::from_array(aligns);

    let soa: Soa<_> = aligns.into_iter().collect();
    assert_eq!(soa, aligns_array.as_slice());
}

#[test]
pub fn iterator_slice_methods() {
    let mut soa = Soa::from(ABCDE);
    let array = ABCDE_SOA;
    let slice = array.as_slice();
    let expected = &slice.get(1..).unwrap();

    {
        let mut iter = soa.iter();
        iter.next();
        assert_eq!(iter.as_slice(), expected);
    }

    {
        let mut iter = soa.iter_mut();
        iter.next();
        assert_eq!(iter.as_slice(), expected);
        assert_eq!(iter.as_mut_slice(), expected);
        assert_eq!(&iter.into_slice(), expected);
    }

    {
        let mut iter = soa.into_iter();
        iter.next();
        assert_eq!(iter.as_slice(), expected);
        assert_eq!(iter.as_mut_slice(), expected);
    }
}

#[test]
fn iterator_size_hint() {
    let soa = Soa::from(ABCDE);
    assert_eq!(soa.iter().size_hint(), (5, Some(5)));
}

#[test]
fn iterator_count() {
    let soa = Soa::from(ABCDE);
    assert_eq!(soa.iter().count(), 5);
}

macro_rules! assert_option_eq {
    ($u:expr, $v:expr) => {
        #[allow(clippy::iter_nth_zero)]
        match ($u, $v) {
            (Some(u), Some(v)) => assert_eq!(u.as_soa_ref(), v.as_soa_ref()),
            (None, None) => {}
            (u, v) => panic!("not equal: {u:?}, {v:?}"),
        }
    };
}

#[test]
fn iterator_last() {
    let soa: Soa<_> = ABCDE.into();
    assert_eq!(soa.into_iter().last(), Some(E));
}

#[test]
fn iterator_nth() {
    let soa: Soa<_> = ABCDE.into_iter().cycle().take(20).collect();
    let vec: Vec<_> = ABCDE.into_iter().cycle().take(20).collect();
    let mut iter_soa = soa.iter();
    let mut iter_vec = vec.iter();
    for i in 0..10 {
        assert_option_eq!(iter_vec.nth(i), iter_soa.nth(i));
    }
}

#[test]
fn iterator_nth_back() {
    let soa: Soa<_> = ABCDE.into_iter().cycle().take(20).collect();
    let vec: Vec<_> = ABCDE.into_iter().cycle().take(20).collect();
    let mut iter_soa = soa.iter();
    let mut iter_vec = vec.iter();
    for i in 0..10 {
        assert_option_eq!(iter_vec.nth_back(i), iter_soa.nth_back(i));
    }
}

#[test]
fn iterator_next_back() {
    let soa: Soa<_> = ABCDE.into();
    let vec: Vec<_> = ABCDE.into();
    let mut soa_iter = soa.iter();
    let mut vec_iter = vec.iter();
    for _ in 0..6 {
        assert_option_eq!(vec_iter.next_back(), soa_iter.next_back());
    }
}

#[test]
fn iterator_fold() {
    fn fold(acc: u64, el: El) -> u64 {
        acc + el.foo + el.bar as u64
    }

    let soa: Soa<_> = ABCDE.into();
    let actual = soa.into_iter().fold(0, fold);
    let expected = ABCDE.into_iter().fold(0, fold);
    assert_eq!(actual, expected);
}

#[test]
fn chunks_exact() {
    let soa: Soa<_> = ABCDE.into_iter().cycle().take(11).collect();
    let mut soa_iter = soa.chunks_exact(4);
    assert_eq!(soa_iter.next(), Some(soa![A, B, C, D].as_slice()));
    assert_eq!(soa_iter.next(), Some(soa![E, A, B, C].as_slice()));
    assert_eq!(soa_iter.next(), None);
    assert_eq!(soa_iter.remainder(), &soa![D, E, A].as_slice());
}

#[test]
fn array_eq() {
    let array = ABCDE_SOA;
    assert_eq!(array.as_slice(), array);
}

#[test]
fn array_slice_mut() {
    let mut array = ABCDE_SOA;
    let mut slice = array.as_mut_slice();
    for item in slice.iter_mut() {
        *item.foo += 10;
        *item.bar += 10;
    }
    let expected = ElArray::from_array(ABCDE.map(|el| El {
        foo: el.foo + 10,
        bar: el.bar + 10,
        baz: el.baz,
    }));
    assert_eq!(slice, expected.as_slice());
}

#[test]
fn slices() {
    let soa = Soa::from(ABCDE);
    let slices = soa.slices();
    assert_eq!(slices.foo, soa.foo());
    assert_eq!(slices.bar, soa.bar());
    assert_eq!(slices.baz, soa.baz());
}

#[test]
fn slices_mut() {
    let mut soa = Soa::from(ABCDE);
    let slices = soa.slices_mut();
    for foo in slices.foo {
        *foo += 10;
    }
    for bar in slices.bar {
        *bar += 10;
    }

    let expected = ABCDE.map(|el| el.foo + 10);
    assert_eq!(soa.foo(), expected);

    let expected = ABCDE.map(|el| el.bar + 10);
    assert_eq!(soa.bar(), expected);
}

#[test]
fn array_with_box() {
    #[derive(Soars)]
    #[soa_array]
    #[soa_derive(PartialEq)]
    struct Example {
        foo: Box<u8>,
    }
    let foo = Box::new(42_u8);
    let x = ExampleArray::from_array([Example { foo }]);
    let s = x.as_slice();
    let _ = s.get(0).unwrap().foo;
}

fn assert_send<T: Send>(_t: T) {}
fn assert_sync<T: Sync>(_t: T) {}

#[test]
fn send_sync() {
    assert_send(soa![A]);
    assert_sync(soa![A]);
}

#[test]
#[allow(unused)]
fn field_type_without_partial_eq() {
    // See https://github.com/tim-harding/soa-rs/issues/10

    struct TypeWithoutPartialEq {
        foo: u8,
    }

    #[derive(Soars)]
    struct Example {
        a: TypeWithoutPartialEq,
    }
}

#[test]
fn serde() {
    #[derive(Soars, serde::Deserialize)]
    #[soa_derive(Debug, PartialEq)]
    #[soa_derive(include(Ref), serde::Serialize)]
    struct Test {
        n: i32,
        s: String,
    }

    let original = soa![
        Test {
            n: 10,
            s: "Hello".to_string()
        },
        Test {
            n: 20,
            s: "Serde".to_string()
        }
    ];

    let serial = serde_json::to_string(&original).unwrap();
    let deserial: Soa<Test> = serde_json::from_str(&serial).unwrap();
    assert_eq!(original, deserial);
}

#[test]
fn mutex() {
    // Regression test for https://github.com/tim-harding/soa-rs/issues/13
    #[derive(Soars)]
    struct M(Mutex<usize>);
}

#[test]
fn no_dead_code_warning() {
    #![deny(dead_code)]

    #[derive(Soars)]
    struct NeverUsed {
        field: i32,
    }
}

// Regression test for
// https://github.com/tim-harding/soa-rs/issues/20
#[test]
fn for_each_double_free() {
    #[derive(Soars)]
    pub struct Example {
        pub content: String,
    }

    let buffer = soa_rs::soa![Example {
        content: "foo".into(),
    }];

    buffer.into_iter().for_each(|_| {});
}

#[test]
fn no_transmute_ptr_to_ptr_warning() {
    #![deny(clippy::transmute_ptr_to_ptr)]

    #[derive(Soars)]
    struct Dummy {
        field: i32,
    }
}

#[test]
fn serde_deserialize_no_realloc() {
    #[derive(Soars, serde::Deserialize)]
    #[soa_derive(Debug, PartialEq)]
    #[soa_derive(include(Ref), serde::Serialize)]
    struct Item {
        x: u32,
        y: u32,
    }

    let original = soa![
        Item { x: 1, y: 2 },
        Item { x: 3, y: 4 },
        Item { x: 5, y: 6 },
    ];

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Soa<Item> = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
    // serde_json does not provide a size_hint (returns None), so capacity
    // will be >= len due to growth factor. Just verify round-trip correctness.
    assert!(deserialized.capacity() >= deserialized.len());
}

#[test]
fn serde_roundtrip_empty() {
    #[derive(Soars, serde::Deserialize)]
    #[soa_derive(Debug, PartialEq)]
    #[soa_derive(include(Ref), serde::Serialize)]
    struct Item {
        x: u32,
    }

    let original: Soa<Item> = Soa::new();
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Soa<Item> = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

// ---- truncate tests ----

#[test]
fn truncate_basic() {
    let mut soa: Soa<El> = [A, B, C, D, E].into();
    soa.truncate(3);
    assert_eq!(soa.len(), 3);
    assert!(soa.capacity() >= 5);
    assert!(soa.into_iter().eq([A, B, C].into_iter()));
}

#[test]
fn truncate_noop_when_len_ge_current() {
    let mut soa: Soa<El> = [A, B].into();
    soa.truncate(5);
    assert_eq!(soa.len(), 2);
    soa.truncate(2);
    assert_eq!(soa.len(), 2);
}

#[test]
fn truncate_to_zero() {
    let mut soa: Soa<El> = [A, B, C].into();
    soa.truncate(0);
    assert!(soa.is_empty());
}

#[test]
fn truncate_drop_count() {
    // SingleDrop panics if dropped more than once — verifies no double-drop.
    let mut soa: Soa<El> = [A, B, C, D, E].into();
    soa.truncate(2);
    assert_eq!(soa.len(), 2);
    // drop of `soa` here drops A and B — passes if no panic
}

// ---- append tests ----

#[test]
fn append_basic() {
    let mut a: Soa<El> = [A, B].into();
    let mut b: Soa<El> = [C, D].into();
    a.append(&mut b);
    assert_eq!(a.len(), 4);
    assert!(b.is_empty());
    assert!(a.into_iter().eq([A, B, C, D].into_iter()));
}

#[test]
fn append_into_empty() {
    let mut a: Soa<El> = Soa::new();
    let mut b: Soa<El> = [A, B].into();
    a.append(&mut b);
    assert_eq!(a.len(), 2);
    assert!(b.is_empty());
    assert!(a.into_iter().eq([A, B].into_iter()));
}

#[test]
fn append_from_empty() {
    let mut a: Soa<El> = [A].into();
    let mut b: Soa<El> = Soa::new();
    a.append(&mut b);
    assert_eq!(a.len(), 1);
    assert!(a.into_iter().eq([A].into_iter()));
}

#[test]
fn append_drop_correctness() {
    // SingleDrop panics on double-drop — verifies no double-free.
    let mut a: Soa<El> = [A, B].into();
    let mut b: Soa<El> = [C, D].into();
    a.append(&mut b);
    assert_eq!(a.len(), 4);
    assert!(b.is_empty());
    // both a and b drop cleanly here
}

#[test]
fn chunks_exact_double_ended() {
    let soa: Soa<El> = [A, B, C, D, E, A].into(); // 6 elements
    let mut iter = soa.chunks_exact(2);
    assert_eq!(iter.next(), Some(soa![A, B].as_slice()));
    assert_eq!(iter.next_back(), Some(soa![E, A].as_slice()));
    assert_eq!(iter.next(), Some(soa![C, D].as_slice()));
    assert_eq!(iter.next_back(), None);
    assert_eq!(iter.next(), None);
}

#[test]
fn chunks_exact_exact_size() {
    let soa: Soa<El> = [A, B, C, D].into();
    let mut iter = soa.chunks_exact(2);
    assert_eq!(iter.len(), 2);
    iter.next();
    assert_eq!(iter.len(), 1);
    iter.next();
    assert_eq!(iter.len(), 0);
}

#[test]
fn chunks_exact_rev() {
    let soa: Soa<El> = [A, B, C, D, E, A].into();
    let chunks: Vec<_> = soa.chunks_exact(2).rev().collect();
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], soa![E, A].as_slice());
    assert_eq!(chunks[1], soa![C, D].as_slice());
    assert_eq!(chunks[2], soa![A, B].as_slice());
}

#[test]
fn chunks_exact_fused() {
    let soa: Soa<El> = [A, B].into();
    let mut iter = soa.chunks_exact(2);
    assert!(iter.next().is_some());
    assert!(iter.next().is_none());
    assert!(iter.next().is_none()); // fused: stays None
}

#[test]
fn soa_macro_repeat_single_alloc() {
    // After the fix, capacity should equal N exactly (no over-allocation).
    let soa: Soa<El> = soa![A; 10];
    assert_eq!(soa.len(), 10);
    assert_eq!(soa.capacity(), 10);
    for i in 0..10 {
        let el = soa.idx(i);
        assert_eq!(el.foo, &A.foo);
        assert_eq!(el.bar, &A.bar);
    }
}

#[test]
fn soa_macro_repeat_zero_still_works() {
    let soa: Soa<El> = soa![A; 0];
    assert!(soa.is_empty());
}

#[test]
fn soa_macro_repeat_one_still_works() {
    let soa: Soa<El> = soa![A; 1];
    assert_eq!(soa.len(), 1);
}

// ---- retain ----

#[test]
fn retain_keep_all() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3)].into();
    soa.retain(|_| true);
    assert_eq!(soa, soa![Foo(1), Foo(2), Foo(3)]);
}

#[test]
fn retain_drop_all() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3)].into();
    soa.retain(|_| false);
    assert!(soa.is_empty());
}

#[test]
fn retain_keep_even() {
    let mut soa: Soa<_> = (0u8..8).map(Foo).collect::<Soa<_>>();
    soa.retain(|r| r.0 % 2 == 0);
    assert_eq!(soa, soa![Foo(0), Foo(2), Foo(4), Foo(6)]);
}

#[test]
fn retain_drop_correctness() {
    let mut soa: Soa<El> = [A, B, C, D, E].into();
    soa.retain(|r| *r.foo % 8 == 0); // keeps A (foo=0), C (foo=8), E (foo=16)
    assert_eq!(soa.len(), 3);
}

#[test]
fn retain_mut_modifies_kept() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3), Foo(4)].into();
    soa.retain_mut(|r| {
        *r.0 *= 10;
        *r.0 < 30
    });
    assert_eq!(soa, soa![Foo(10), Foo(20)]);
}

// ---- dedup ----

#[test]
fn dedup_no_duplicates() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3)].into();
    soa.dedup();
    assert_eq!(soa, soa![Foo(1), Foo(2), Foo(3)]);
}

#[test]
fn dedup_all_same() {
    let mut soa: Soa<_> = [Foo(5), Foo(5), Foo(5)].into();
    soa.dedup();
    assert_eq!(soa, soa![Foo(5)]);
}

#[test]
fn dedup_runs() {
    let mut soa: Soa<_> = [Foo(1), Foo(1), Foo(2), Foo(3), Foo(3), Foo(3), Foo(2)].into();
    soa.dedup();
    assert_eq!(soa, soa![Foo(1), Foo(2), Foo(3), Foo(2)]);
}

#[test]
fn dedup_empty() {
    let mut soa: Soa<Foo> = Soa::new();
    soa.dedup();
    assert!(soa.is_empty());
}

#[test]
fn dedup_single() {
    let mut soa = soa![Foo(42)];
    soa.dedup();
    assert_eq!(soa, soa![Foo(42)]);
}

#[test]
fn dedup_by_custom() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(10), Foo(11), Foo(20)].into();
    soa.dedup_by(|a, b| a.0 / 10 == b.0 / 10);
    assert_eq!(soa, soa![Foo(1), Foo(10), Foo(20)]);
}

#[test]
fn dedup_drop_correctness() {
    let mut soa: Soa<El> = [A, A, B, B, C].into();
    soa.dedup();
    assert_eq!(soa.len(), 3);
}

// ---- dedup_by_key ----

#[test]
fn dedup_by_key_basic() {
    let mut soa: Soa<_> = soa![
        ExtraImplTester { things: 1, stuff: 10 },
        ExtraImplTester { things: 1, stuff: 20 },
        ExtraImplTester { things: 2, stuff: 30 },
        ExtraImplTester { things: 2, stuff: 40 },
        ExtraImplTester { things: 3, stuff: 50 },
    ];
    soa.dedup_by_key(|r| *r.things);
    assert_eq!(soa.len(), 3);
    assert_eq!(soa.things(), &[1, 2, 3]);
}

#[test]
fn dedup_by_key_no_dups() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3)].into();
    soa.dedup_by_key(|r| *r.0);
    assert_eq!(soa, soa![Foo(1), Foo(2), Foo(3)]);
}

// ---- drain ----

#[test]
fn drain_full_range() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3)].into();
    let drained: Vec<_> = soa.drain(..).collect();
    assert_eq!(drained, vec![Foo(1), Foo(2), Foo(3)]);
    assert!(soa.is_empty());
}

#[test]
fn drain_middle() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3), Foo(4), Foo(5)].into();
    let drained: Vec<_> = soa.drain(1..4).collect();
    assert_eq!(drained, vec![Foo(2), Foo(3), Foo(4)]);
    assert_eq!(soa, soa![Foo(1), Foo(5)]);
}

#[test]
fn drain_prefix() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3)].into();
    let drained: Vec<_> = soa.drain(..2).collect();
    assert_eq!(drained, vec![Foo(1), Foo(2)]);
    assert_eq!(soa, soa![Foo(3)]);
}

#[test]
fn drain_suffix() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3)].into();
    let drained: Vec<_> = soa.drain(1..).collect();
    assert_eq!(drained, vec![Foo(2), Foo(3)]);
    assert_eq!(soa, soa![Foo(1)]);
}

#[test]
fn drain_empty_range() {
    let mut soa: Soa<_> = [Foo(1), Foo(2)].into();
    let drained: Vec<_> = soa.drain(1..1).collect();
    assert!(drained.is_empty());
    assert_eq!(soa, soa![Foo(1), Foo(2)]);
}

#[test]
fn drain_partial_consume_then_drop() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3), Foo(4), Foo(5)].into();
    {
        let mut drain = soa.drain(1..4);
        assert_eq!(drain.next(), Some(Foo(2)));
        // Drop drain without consuming Foo(3) and Foo(4).
    }
    assert_eq!(soa, soa![Foo(1), Foo(5)]);
}

#[test]
fn drain_drop_correctness() {
    let mut soa: Soa<El> = [A, B, C, D, E].into();
    let _drained: Vec<_> = soa.drain(1..3).collect();
    assert_eq!(soa.len(), 3);
}

#[test]
fn drain_size_hint() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3), Foo(4)].into();
    let mut drain = soa.drain(..);
    assert_eq!(drain.size_hint(), (4, Some(4)));
    drain.next();
    assert_eq!(drain.size_hint(), (3, Some(3)));
}

// ---- split_off ----

#[test]
fn split_off_middle() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3), Foo(4)].into();
    let tail = soa.split_off(2);
    assert_eq!(soa, soa![Foo(1), Foo(2)]);
    assert_eq!(tail, soa![Foo(3), Foo(4)]);
}

#[test]
fn split_off_at_zero() {
    let mut soa: Soa<_> = [Foo(1), Foo(2)].into();
    let tail = soa.split_off(0);
    assert!(soa.is_empty());
    assert_eq!(tail, soa![Foo(1), Foo(2)]);
}

#[test]
fn split_off_at_len() {
    let mut soa: Soa<_> = [Foo(1), Foo(2)].into();
    let tail = soa.split_off(2);
    assert_eq!(soa, soa![Foo(1), Foo(2)]);
    assert!(tail.is_empty());
}

#[test]
fn split_off_drop_correctness() {
    let mut soa: Soa<El> = [A, B, C, D, E].into();
    let tail = soa.split_off(2);
    assert_eq!(soa.len(), 2);
    assert_eq!(tail.len(), 3);
}

#[test]
#[should_panic]
fn split_off_out_of_bounds() {
    let mut soa: Soa<_> = [Foo(1)].into();
    soa.split_off(5);
}

// ---- resize ----

#[test]
fn resize_grow() {
    let mut soa: Soa<_> = [Foo(1), Foo(2)].into();
    soa.resize(5, Foo(0));
    assert_eq!(soa, soa![Foo(1), Foo(2), Foo(0), Foo(0), Foo(0)]);
}

#[test]
fn resize_shrink() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3), Foo(4)].into();
    soa.resize(2, Foo(0));
    assert_eq!(soa, soa![Foo(1), Foo(2)]);
}

#[test]
fn resize_noop() {
    let mut soa: Soa<_> = [Foo(1), Foo(2)].into();
    soa.resize(2, Foo(99));
    assert_eq!(soa, soa![Foo(1), Foo(2)]);
}

#[test]
fn resize_to_zero() {
    let mut soa: Soa<_> = [Foo(1), Foo(2), Foo(3)].into();
    soa.resize(0, Foo(0));
    assert!(soa.is_empty());
}

#[test]
fn resize_with_counter() {
    let mut soa: Soa<Foo> = Soa::new();
    let mut counter = 0u8;
    soa.resize_with(4, || {
        let v = Foo(counter);
        counter += 1;
        v
    });
    assert_eq!(soa, soa![Foo(0), Foo(1), Foo(2), Foo(3)]);
}

#[test]
fn resize_drop_correctness() {
    let mut soa: Soa<El> = [A, B, C, D, E].into();
    soa.resize(2, A);
    assert_eq!(soa.len(), 2);
}

#[test]
fn sort_indices_by_basic() {
    let soa = soa![Foo(3), Foo(1), Foo(4), Foo(1), Foo(5)];
    let order = soa.sort_indices_by(|a, b| a.0.cmp(b.0));
    let sorted: Vec<u8> = order.iter().map(|&i| *soa.idx(i).0).collect();
    assert_eq!(sorted, [1, 1, 3, 4, 5]);
}

#[test]
fn sort_indices_by_key_basic() {
    let soa = soa![Foo(3), Foo(1), Foo(4), Foo(1), Foo(5)];
    let order = soa.sort_indices_by_key(|r| *r.0);
    let sorted: Vec<u8> = order.iter().map(|&i| *soa.idx(i).0).collect();
    assert_eq!(sorted, [1, 1, 3, 4, 5]);
}

#[test]
fn sort_indices_stable() {
    let soa = soa![Foo(1), Foo(1), Foo(1)];
    let order = soa.sort_indices_by_key(|r| *r.0);
    assert_eq!(order, [0, 1, 2]);
}

#[test]
fn sort_indices_empty() {
    let soa: Soa<Foo> = Soa::new();
    let order = soa.sort_indices_by(|a, b| a.0.cmp(b.0));
    assert!(order.is_empty());
}

#[test]
fn sort_indices_single() {
    let soa = soa![Foo(42)];
    let order = soa.sort_indices_by_key(|r| *r.0);
    assert_eq!(order, [0]);
}
