use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

use ordered_float::OrderedFloat;

/// Used to store keys values for command modifiers. Heavily inspired by
/// serde-value.
#[derive(Clone, Debug)]
pub enum ModifierValue {
    Bool(bool),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),

    F32(f32),
    F64(f64),

    Char(char),
    String(String),

    Unit,
    Option(Option<Box<ModifierValue>>),
    Seq(Vec<ModifierValue>),
    Map(BTreeMap<ModifierValue, ModifierValue>),
}

#[allow(clippy::unit_hash)]
impl Hash for ModifierValue {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.discriminant().hash(hasher);
        match self {
            ModifierValue::Bool(v) => v.hash(hasher),
            ModifierValue::U8(v) => v.hash(hasher),
            ModifierValue::U16(v) => v.hash(hasher),
            ModifierValue::U32(v) => v.hash(hasher),
            ModifierValue::U64(v) => v.hash(hasher),
            ModifierValue::I8(v) => v.hash(hasher),
            ModifierValue::I16(v) => v.hash(hasher),
            ModifierValue::I32(v) => v.hash(hasher),
            ModifierValue::I64(v) => v.hash(hasher),
            ModifierValue::F32(v) => OrderedFloat(*v).hash(hasher),
            ModifierValue::F64(v) => OrderedFloat(*v).hash(hasher),
            ModifierValue::Char(v) => v.hash(hasher),
            ModifierValue::String(v) => v.hash(hasher),
            ModifierValue::Unit => ().hash(hasher),
            ModifierValue::Option(v) => v.hash(hasher),
            ModifierValue::Seq(v) => v.hash(hasher),
            ModifierValue::Map(v) => v.hash(hasher),
        }
    }
}

impl PartialEq for ModifierValue {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (&ModifierValue::Bool(v0), &ModifierValue::Bool(v1)) if v0 == v1 => true,
            (&ModifierValue::U8(v0), &ModifierValue::U8(v1)) if v0 == v1 => true,
            (&ModifierValue::U16(v0), &ModifierValue::U16(v1)) if v0 == v1 => true,
            (&ModifierValue::U32(v0), &ModifierValue::U32(v1)) if v0 == v1 => true,
            (&ModifierValue::U64(v0), &ModifierValue::U64(v1)) if v0 == v1 => true,
            (&ModifierValue::I8(v0), &ModifierValue::I8(v1)) if v0 == v1 => true,
            (&ModifierValue::I16(v0), &ModifierValue::I16(v1)) if v0 == v1 => true,
            (&ModifierValue::I32(v0), &ModifierValue::I32(v1)) if v0 == v1 => true,
            (&ModifierValue::I64(v0), &ModifierValue::I64(v1)) if v0 == v1 => true,
            (&ModifierValue::F32(v0), &ModifierValue::F32(v1))
                if OrderedFloat(v0) == OrderedFloat(v1) =>
            {
                true
            }
            (&ModifierValue::F64(v0), &ModifierValue::F64(v1))
                if OrderedFloat(v0) == OrderedFloat(v1) =>
            {
                true
            }
            (&ModifierValue::Char(v0), &ModifierValue::Char(v1)) if v0 == v1 => true,
            (ModifierValue::String(v0), ModifierValue::String(v1)) if v0 == v1 => true,
            (ModifierValue::Unit, ModifierValue::Unit) => true,
            (ModifierValue::Option(v0), ModifierValue::Option(v1)) if v0 == v1 => true,
            (ModifierValue::Seq(v0), ModifierValue::Seq(v1)) if v0 == v1 => true,
            (ModifierValue::Map(v0), ModifierValue::Map(v1)) if v0 == v1 => true,
            _ => false,
        }
    }
}

impl Ord for ModifierValue {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match (self, rhs) {
            (&ModifierValue::Bool(v0), ModifierValue::Bool(v1)) => v0.cmp(v1),
            (&ModifierValue::U8(v0), ModifierValue::U8(v1)) => v0.cmp(v1),
            (&ModifierValue::U16(v0), ModifierValue::U16(v1)) => v0.cmp(v1),
            (&ModifierValue::U32(v0), ModifierValue::U32(v1)) => v0.cmp(v1),
            (&ModifierValue::U64(v0), ModifierValue::U64(v1)) => v0.cmp(v1),
            (&ModifierValue::I8(v0), ModifierValue::I8(v1)) => v0.cmp(v1),
            (&ModifierValue::I16(v0), ModifierValue::I16(v1)) => v0.cmp(v1),
            (&ModifierValue::I32(v0), ModifierValue::I32(v1)) => v0.cmp(v1),
            (&ModifierValue::I64(v0), ModifierValue::I64(v1)) => v0.cmp(v1),
            (&ModifierValue::F32(v0), &ModifierValue::F32(v1)) => {
                OrderedFloat(v0).cmp(&OrderedFloat(v1))
            }
            (&ModifierValue::F64(v0), &ModifierValue::F64(v1)) => {
                OrderedFloat(v0).cmp(&OrderedFloat(v1))
            }
            (ModifierValue::Char(v0), ModifierValue::Char(v1)) => v0.cmp(v1),
            (ModifierValue::String(v0), ModifierValue::String(v1)) => v0.cmp(v1),
            (ModifierValue::Unit, &ModifierValue::Unit) => Ordering::Equal,
            (ModifierValue::Option(v0), ModifierValue::Option(v1)) => v0.cmp(v1),
            (ModifierValue::Seq(v0), ModifierValue::Seq(v1)) => v0.cmp(v1),
            (ModifierValue::Map(v0), ModifierValue::Map(v1)) => v0.cmp(v1),
            (v0, v1) => v0.discriminant().cmp(&v1.discriminant()),
        }
    }
}

impl ModifierValue {
    fn discriminant(&self) -> usize {
        match *self {
            ModifierValue::Bool(..) => 0,
            ModifierValue::U8(..) => 1,
            ModifierValue::U16(..) => 2,
            ModifierValue::U32(..) => 3,
            ModifierValue::U64(..) => 4,
            ModifierValue::I8(..) => 5,
            ModifierValue::I16(..) => 6,
            ModifierValue::I32(..) => 7,
            ModifierValue::I64(..) => 8,
            ModifierValue::F32(..) => 9,
            ModifierValue::F64(..) => 10,
            ModifierValue::Char(..) => 11,
            ModifierValue::String(..) => 12,
            ModifierValue::Unit => 13,
            ModifierValue::Option(..) => 14,
            ModifierValue::Seq(..) => 16,
            ModifierValue::Map(..) => 17,
        }
    }
}

impl Eq for ModifierValue {}
impl PartialOrd for ModifierValue {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

macro_rules! impl_from {
    ($ty:ty, $variant:ident) => {
        impl From<$ty> for ModifierValue {
            fn from(v: $ty) -> Self {
                ModifierValue::$variant(v)
            }
        }
    };
}

impl_from!(bool, Bool);

impl_from!(u8, U8);
impl_from!(u16, U16);
impl_from!(u32, U32);
impl_from!(u64, U64);

impl_from!(i8, I8);
impl_from!(i16, I16);
impl_from!(i32, I32);
impl_from!(i64, I64);

impl_from!(f32, F32);
impl_from!(f64, F64);

impl_from!(char, Char);
impl_from!(String, String);

impl From<&str> for ModifierValue {
    fn from(v: &str) -> Self {
        ModifierValue::String(v.to_owned())
    }
}

impl From<()> for ModifierValue {
    fn from((): ()) -> Self {
        ModifierValue::Unit
    }
}

impl From<Option<ModifierValue>> for ModifierValue {
    fn from(v: Option<ModifierValue>) -> Self {
        ModifierValue::Option(v.map(Box::new))
    }
}

impl<T: Into<ModifierValue>> From<Vec<T>> for ModifierValue {
    fn from(v: Vec<T>) -> Self {
        ModifierValue::Seq(v.into_iter().map(Into::into).collect())
    }
}

impl<K: Into<ModifierValue>, V: Into<ModifierValue>> From<BTreeMap<K, V>> for ModifierValue {
    fn from(v: BTreeMap<K, V>) -> Self {
        ModifierValue::Map(v.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

impl<K: Into<ModifierValue>, V: Into<ModifierValue>> From<HashMap<K, V>> for ModifierValue {
    fn from(v: HashMap<K, V>) -> Self {
        ModifierValue::Map(v.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}
