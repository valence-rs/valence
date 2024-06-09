use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{BuildHasher, Hash};
use std::io::Write;

use anyhow::ensure;

use crate::impls::cautious_capacity;
use crate::{Decode, Encode, VarInt};

impl<T> Encode for BTreeSet<T>
where
    T: Encode,
{
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let len = self.len();

        ensure!(
            i32::try_from(len).is_ok(),
            "length of B-tree set ({len}) exceeds i32::MAX"
        );

        VarInt(len as i32).encode(&mut w)?;

        for val in self {
            val.encode(&mut w)?;
        }

        Ok(())
    }
}

impl<'a, T> Decode<'a> for BTreeSet<T>
where
    T: Ord + Decode<'a>,
{
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(
            len >= 0,
            "attempt to decode B-tree set with negative length"
        );
        let len = len as usize;

        let mut set = BTreeSet::new();

        for _ in 0..len {
            ensure!(
                set.insert(T::decode(r)?),
                "encountered duplicate item while decoding B-tree set"
            );
        }

        Ok(set)
    }
}

impl<T, S> Encode for HashSet<T, S>
where
    T: Encode,
{
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let len = self.len();

        ensure!(
            i32::try_from(len).is_ok(),
            "length of hash set ({len}) exceeds i32::MAX"
        );

        VarInt(len as i32).encode(&mut w)?;

        for val in self {
            val.encode(&mut w)?;
        }

        Ok(())
    }
}

impl<'a, T, S> Decode<'a> for HashSet<T, S>
where
    T: Eq + Hash + Decode<'a>,
    S: BuildHasher + Default,
{
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(len >= 0, "attempt to decode hash set with negative length");
        let len = len as usize;

        let mut set = HashSet::with_capacity_and_hasher(cautious_capacity::<T>(len), S::default());

        for _ in 0..len {
            ensure!(
                set.insert(T::decode(r)?),
                "encountered duplicate item while decoding hash set"
            );
        }

        Ok(set)
    }
}

impl<K, V> Encode for BTreeMap<K, V>
where
    K: Encode,
    V: Encode,
{
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let len = self.len();

        ensure!(
            i32::try_from(len).is_ok(),
            "length of B-tree map ({len}) exceeds i32::MAX"
        );

        VarInt(len as i32).encode(&mut w)?;

        for pair in self {
            pair.encode(&mut w)?;
        }

        Ok(())
    }
}

impl<'a, K, V> Decode<'a> for BTreeMap<K, V>
where
    K: Ord + Decode<'a>,
    V: Decode<'a>,
{
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(
            len >= 0,
            "attempt to decode B-tree map with negative length"
        );
        let len = len as usize;

        let mut map = BTreeMap::new();

        for _ in 0..len {
            ensure!(
                map.insert(K::decode(r)?, V::decode(r)?).is_none(),
                "encountered duplicate key while decoding B-tree map"
            );
        }

        Ok(map)
    }
}

impl<K, V, S> Encode for HashMap<K, V, S>
where
    K: Encode,
    V: Encode,
{
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let len = self.len();

        ensure!(
            i32::try_from(len).is_ok(),
            "length of hash map ({len}) exceeds i32::MAX"
        );

        VarInt(len as i32).encode(&mut w)?;

        for pair in self {
            pair.encode(&mut w)?;
        }

        Ok(())
    }
}

impl<'a, K, V, S> Decode<'a> for HashMap<K, V, S>
where
    K: Eq + Hash + Decode<'a>,
    V: Decode<'a>,
    S: BuildHasher + Default,
{
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(len >= 0, "attempt to decode hash map with negative length");
        let len = len as usize;

        let mut map =
            HashMap::with_capacity_and_hasher(cautious_capacity::<(K, V)>(len), S::default());

        for _ in 0..len {
            ensure!(
                map.insert(K::decode(r)?, V::decode(r)?).is_none(),
                "encountered duplicate item while decoding hash map"
            );
        }

        Ok(map)
    }
}
