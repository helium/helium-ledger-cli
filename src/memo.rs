use crate::{Error, Result};
use helium_wallet::traits::B64;
use serde::de::{self, Deserialize, Deserializer, Visitor};
use std::{fmt, str::FromStr};

#[derive(Debug, Default, PartialEq)]
pub struct Memo(u64);

impl FromStr for Memo {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let decoded = base64::decode(s)?;
        if decoded.len() != 8 {
            return Err(Error::from_b64());
        }
        let bytes = [
            decoded[0], decoded[1], decoded[2], decoded[3], decoded[4], decoded[5], decoded[6],
            decoded[7],
        ];
        Ok(Memo(u64::from_le_bytes(bytes)))
    }
}

impl From<u64> for Memo {
    fn from(v: u64) -> Self {
        Memo(v)
    }
}

impl From<&Memo> for u64 {
    fn from(v: &Memo) -> Self {
        v.0
    }
}

impl fmt::Display for Memo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_b64().unwrap())
    }
}

impl<'de> Deserialize<'de> for Memo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MemoVisitor;

        impl<'de> Visitor<'de> for MemoVisitor {
            type Value = Memo;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("base64 string")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Memo, E>
            where
                E: de::Error,
            {
                match u64::from_b64(value) {
                    Ok(v) => Ok(Memo(v)),
                    Err(_) => Err(de::Error::custom("invalid memo")),
                }
            }
        }

        deserializer.deserialize_str(MemoVisitor)
    }
}
