use serde::{Deserialize, Serialize};

use super::random_value::{InvalidLengthError, RandomValue};

const IDLEN: usize = 21;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientId {
    inner: RandomValue<IDLEN>,
}

impl ClientId {
    pub fn new() -> Self {
        Self {
            inner: nanoid::nanoid!().parse().unwrap(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::default::Default for ClientId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.as_str())
    }
}

impl std::str::FromStr for ClientId {
    type Err = InvalidLengthError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let inner: RandomValue<IDLEN> = src.parse()?;

        Ok(Self { inner })
    }
}

impl TryFrom<&[u8]> for ClientId {
    type Error = InvalidLengthError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let inner = RandomValue::from_bytes(value)?;

        Ok(Self { inner })
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientId, IDLEN};

    #[test]
    fn client_id_length() {
        let id = ClientId::new();
        assert_eq!(id.len(), IDLEN, "length of client Id is {IDLEN}");
        assert!(!id.is_empty(), "is is NOT empty");

        let str_id = id.as_str();
        assert_eq!(str_id.len(), IDLEN, "length of client Id str is {IDLEN}");
        assert!(
            str_id.len() <= 256,
            "client Id string is less than 256 characters"
        );

        assert_eq!(
            id.to_string(),
            str_id,
            "equality of string and str versions of id"
        );
    }
}
