use std::mem;

use crate::tag::Tag;
use crate::{compound, from_binary_slice, to_binary_writer, Compound, List, Value};

const ROOT_NAME: &str = "The root name‽";

#[test]
fn round_trip() {
    let mut buf = Vec::new();

    let compound = example_compound();

    to_binary_writer(&mut buf, &compound, ROOT_NAME).unwrap();

    println!("{buf:?}");

    let (decoded, root_name) = from_binary_slice(&mut buf.as_slice()).unwrap();

    assert_eq!(root_name, ROOT_NAME);
    assert_eq!(compound, decoded);
}

#[test]
fn check_min_sizes() {
    fn check(min_val: Value, expected_size: usize) {
        /// TAG_Compound + root name + field tag + field name + TAG_End
        const COMPOUND_OVERHEAD: usize = 1 + 2 + 1 + 2 + 1;

        let dbg = format!("{min_val:?}");
        let mut buf = Vec::new();

        to_binary_writer(&mut buf, &compound!("" => min_val), "").unwrap();

        assert_eq!(
            expected_size,
            buf.len() - COMPOUND_OVERHEAD,
            "size mismatch for {dbg}"
        );
    }

    check(Value::Byte(0), 1);
    check(Value::Short(0), 2);
    check(Value::Int(0), 4);
    check(Value::Long(0), 8);
    check(Value::Float(0.0), 4);
    check(Value::Double(0.0), 8);
    check(Value::ByteArray([].into()), 4);
    check(Value::String("".into()), 2);
    check(Value::List(Vec::<i32>::new().into()), 5);
    check(Value::Compound(compound!()), 1);
    check(Value::IntArray([].into()), 4);
    check(Value::LongArray([].into()), 4);
}

#[test]
fn deeply_nested_compound_encode() {
    let mut c = compound!("" => 111_i8);
    for _ in 0..10_000 {
        c = compound!("" => c);
    }

    // Should not overflow the stack
    let _ = to_binary_writer(&mut Vec::new(), &c, ROOT_NAME);

    // Don"t overflow the stack while dropping.
    mem::forget(c);
}

#[test]
fn deeply_nested_compound_decode() {
    let mut buf = vec![Tag::Compound as u8, 0, 0]; // Root compound
    let n = 10_000;

    for _ in 0..n {
        buf.extend([Tag::Compound as u8, 0, 0]);
    }

    buf.extend((0..n).map(|_| Tag::End as u8));

    buf.push(Tag::End as u8); // End root compound

    // Should not overflow the stack
    let _ = from_binary_slice(&mut buf.as_slice());
}

#[test]
fn deeply_nested_list_encode() {
    let mut l = List::Byte(Vec::new());
    for _ in 0..10_000 {
        l = List::List(vec![l]);
    }

    let c = compound!("" => l);

    // Should not panic
    let _ = to_binary_writer(&mut Vec::new(), &c, ROOT_NAME);

    // Don"t overflow the stack while dropping.
    mem::forget(c);
}

#[test]
fn deeply_nested_list_decode() {
    // Root compound with one field.
    let mut buf = vec![Tag::Compound as u8, 0, 0, Tag::List as u8, 0, 0];
    let n = 10_000;

    for _ in 0..n - 1 {
        buf.extend([Tag::List as u8, 0, 0, 0, 1]); // List of list
    }

    // Last list is an empty list of bytes.
    buf.extend([Tag::Byte as u8, 0, 0, 0, 0]);

    buf.push(Tag::End as u8); // End root compound

    // Should not overflow the stack
    let _ = from_binary_slice(&mut buf.as_slice());
}

#[cfg(feature = "preserve_order")]
#[test]
fn preserves_order() {
    let letters = ["g", "b", "d", "e", "h", "z", "m", "a", "q"];

    let mut c = Compound::new();
    for l in letters {
        c.insert(l, 0_i8);
    }

    for (k, l) in c.keys().zip(letters) {
        assert_eq!(k, l);
    }
}

fn example_compound() -> Compound {
    fn inner() -> Compound {
        compound! {
            "int" => i32::MIN,
            "long" => i64::MAX,
            "float" => 1e10_f32,
            "double" => f64::INFINITY,
        }
    }

    compound! {
        "byte" => 123_i8,
        "list_of_int" => List::Int(vec![3, -7, 5]),
        "list_of_string" => List::String(vec![
            "foo".to_owned(),
            "bar".to_owned(),
            "baz".to_owned()
        ]),
        "string" => "aé日",
        "compound" => inner(),
        "list_of_compound" => List::Compound(vec![
            inner(),
            inner(),
            inner(),
        ]),
        "int_array" => vec![5, -9, i32::MIN, 0, i32::MAX],
        "byte_array" => vec![0_i8, 2, 3],
        "long_array" => vec![123_i64, 456, 789],
    }
}
