// use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod client;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Id<const L: usize> {
    inner: [u8; L],
}

// impl<const L: usize> Id<L> {
//     const LENGTH: usize = L;

//     pub fn as_bytes(&self) -> [u8; L] {
//         self.inner
//     }

//     pub fn as_str(&self) -> &str {
//         std::str::from_utf8(&self.inner).unwrap()
//     }

//     pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidLengthError> {
//         Ok(Self {
//             inner: bytes.try_into().map_err(|_| InvalidLengthError {
//                 expected: L,
//                 actual: bytes.len(),
//             })?,
//         })
//     }
// }

// impl<const L: usize> std::str::FromStr for Id<L> {
//     type Err = InvalidLengthError;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         Self::from_bytes(s.as_bytes())
//     }
// }

// impl<const L: usize> Serialize for Id<L> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         serializer
//             .serialize_str(std::str::from_utf8(&self.inner).map_err(serde::ser::Error::custom)?)
//     }
// }

// impl<'de, const L: usize> Deserialize<'de> for Id<L> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         Self::from_bytes(String::deserialize(deserializer)?.as_bytes())
//             .map_err(serde::de::Error::custom)
//     }
// }

#[derive(Debug)]
pub struct InvalidLengthError {
    pub expected: usize,
    pub actual: usize,
}

impl std::fmt::Display for InvalidLengthError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "invalid id length, expected: {}, actual: {}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for InvalidLengthError {}

pub(crate) use crate::domain::UserId;
