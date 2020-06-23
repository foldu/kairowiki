use serde::{
    de::{Deserializer, Error, Visitor},
    Deserialize,
};
use std::{fmt::Display, marker::PhantomData, str::FromStr};

pub struct HexEncode<T>(pub T);

impl<T> serde::Serialize for HexEncode<T>
where
    T: AsRef<[u8]>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex = hex::encode(self.0.as_ref());
        serializer.serialize_str(&hex)
    }
}

pub fn deserialize_oid<'de, D>(deserializer: D) -> Result<Option<git2::Oid>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s {
        Some(s) => git2::Oid::from_str(&s).map(Some).map_err(D::Error::custom),
        None => Ok(None),
    }
}

pub struct SeparatedList<T>(pub Vec<T>);

struct SeparatedListVisitor<T> {
    _ty: std::marker::PhantomData<T>,
}

impl<T, SE> Visitor<'_> for SeparatedListVisitor<T>
where
    SE: Display,
    T: FromStr<Err = SE>,
{
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A comma separated list")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.split(",")
            .map(|s| s.trim().parse().map_err(Error::custom))
            .collect()
    }
}

impl<'de, T, SE> Deserialize<'de> for SeparatedList<T>
where
    SE: Display,
    T: FromStr<Err = SE>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_str(SeparatedListVisitor {
                _ty: PhantomData::<T>,
            })
            .map(Self)
    }
}
