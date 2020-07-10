use serde::{
    de::{Deserializer, Error, Visitor},
    Deserialize,
};
use std::{fmt::Display, marker::PhantomData, str::FromStr};

#[derive(Copy, Clone)]
pub struct Oid(pub git2::Oid);

impl serde::Serialize for Oid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex = hex::encode(self.0.as_ref());
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for Oid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OidVisitor;
        impl Visitor<'_> for OidVisitor {
            type Value = Oid;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A comma separated list")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                s.parse::<git2::Oid>().map_err(Error::custom).map(Oid)
            }
        }

        deserializer.deserialize_str(OidVisitor)
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

#[test]
fn roundtrip_oid() {
    let oid = "3fc1961eb2ce860a1c05b4cd6a36ca9521127e78"
        .parse::<git2::Oid>()
        .unwrap();
    #[derive(Deserialize, serde::Serialize)]
    struct Test {
        field: Oid,
    }
    let serialized = serde_json::to_string(&Test { field: Oid(oid) }).unwrap();
    let deserialized: Test = serde_json::from_str(&serialized).unwrap();
    assert_eq!(oid, deserialized.field.0);
}

