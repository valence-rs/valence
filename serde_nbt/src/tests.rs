use binary::{from_reader, to_writer, Deserializer, Serializer};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

use super::*;

const ROOT_NAME: &str = "The root name‽";

#[derive(PartialEq, Debug, Serialize, Deserialize)]
struct Struct {
    byte: i8,
    list_of_int: Vec<i32>,
    list_of_string: Vec<String>,
    string: String,
    inner: Inner,
    #[serde(serialize_with = "int_array")]
    int_array: Vec<i32>,
    #[serde(serialize_with = "byte_array")]
    byte_array: Vec<i8>,
    #[serde(serialize_with = "long_array")]
    long_array: Vec<i64>,
}

impl Struct {
    pub fn new() -> Self {
        Self {
            byte: 123,
            list_of_int: vec![3, -7, 5],
            list_of_string: vec!["foo".to_owned(), "bar".to_owned(), "baz".to_owned()],
            string: "aé日".to_owned(),
            inner: Inner {
                int: i32::MIN,
                long: i64::MAX,
                nan_float: OrderedFloat(f32::NAN),
                neg_inf_double: f64::NEG_INFINITY,
            },
            int_array: vec![5, -9, i32::MIN, 0, i32::MAX],
            byte_array: vec![0, 1, 2],
            long_array: vec![123, 456, 789],
        }
    }

    pub fn value() -> Value {
        Value::Compound(
            Compound::from_iter([
                ("byte".into(), 123.into()),
                ("list_of_int".into(), List::Int(vec![3, -7, 5]).into()),
                (
                    "list_of_string".into(),
                    List::String(vec!["foo".into(), "bar".into(), "baz".into()]).into(),
                ),
                ("string".into(), "aé日".into()),
                (
                    "inner".into(),
                    Compound::from_iter([
                        ("int".into(), i32::MIN.into()),
                        ("long".into(), i64::MAX.into()),
                        ("nan_float".into(), f32::NAN.into()),
                        ("neg_inf_double".into(), f64::NEG_INFINITY.into()),
                    ])
                    .into(),
                ),
                (
                    "int_array".into(),
                    vec![5, -9, i32::MIN, 0, i32::MAX].into(),
                ),
                ("byte_array".into(), vec![0_i8, 1, 2].into()),
                ("long_array".into(), vec![123_i64, 456, 789].into()),
            ])
            .into(),
        )
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
struct Inner {
    int: i32,
    long: i64,
    nan_float: OrderedFloat<f32>,
    neg_inf_double: f64,
}

#[test]
fn round_trip() {
    let struct_ = Struct::new();

    let mut buf = Vec::new();
    struct_
        .serialize(&mut Serializer::new(&mut buf, ROOT_NAME))
        .unwrap();

    let reader = &mut buf.as_slice();

    let mut de = Deserializer::new(reader, true);

    let example_de = Struct::deserialize(&mut de).unwrap();

    assert_eq!(struct_, example_de);

    let (_, root) = de.into_inner();

    assert_eq!(root.unwrap(), ROOT_NAME);
}

#[test]
fn serialize() {
    let struct_ = Struct::new();

    let mut buf = Vec::new();

    struct_
        .serialize(&mut Serializer::new(&mut buf, ROOT_NAME))
        .unwrap();

    let example_de: Struct = nbt::from_reader(&mut buf.as_slice()).unwrap();

    assert_eq!(struct_, example_de);
}

#[test]
fn root_requires_compound() {
    let mut buf = Vec::new();
    assert!(123
        .serialize(&mut Serializer::new(&mut buf, ROOT_NAME))
        .is_err());
}

#[test]
fn invalid_array_element() {
    #[derive(Serialize)]
    struct Struct {
        #[serde(serialize_with = "byte_array")]
        data: Vec<i32>,
    }

    let struct_ = Struct {
        data: vec![1, 2, 3],
    };

    let mut buf = Vec::new();
    assert!(struct_
        .serialize(&mut Serializer::new(&mut buf, ROOT_NAME))
        .is_err());
}

// #[test]
// fn struct_to_value() {
//     let mut buf = Vec::new();
//
//     to_writer(&mut buf, ROOT_NAME, &Struct::new()).unwrap();
//
//     let reader = &mut buf.as_slice();
//
//     let val: Value = from_reader(reader).unwrap();
//
//     eprintln!("{:#?}", Struct::value());
//
//     assert_eq!(val, Struct::value());
// }

#[test]
fn value_to_struct() {
    let mut buf = Vec::new();

    to_writer(&mut buf, ROOT_NAME, &Struct::value()).unwrap();

    let reader = &mut buf.as_slice();

    let struct_: Struct = from_reader(reader).unwrap();

    assert_eq!(struct_, Struct::new());
}
