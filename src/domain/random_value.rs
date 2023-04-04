use secrecy::Zeroize;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Zeroize)]
pub struct RandomValue<const L: usize> {
    inner: [u8; L],
}

impl<const L: usize> RandomValue<L> {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.inner).unwrap()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidLengthError> {
        Ok(Self {
            inner: bytes.try_into().map_err(|_| InvalidLengthError {
                expected: L,
                actual: bytes.len(),
            })?,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<const L: usize> std::str::FromStr for RandomValue<L> {
    type Err = InvalidLengthError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_bytes(s.as_bytes())
    }
}

impl<const L: usize> Serialize for RandomValue<L> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer
            .serialize_str(std::str::from_utf8(&self.inner).map_err(serde::ser::Error::custom)?)
    }
}

impl<'de, const L: usize> Deserialize<'de> for RandomValue<L> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::from_bytes(String::deserialize(deserializer)?.as_bytes())
            .map_err(serde::de::Error::custom)
    }
}

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

#[cfg(test)]
mod tests {

    use super::RandomValue;
    use std::str::FromStr;

    #[test]
    fn id_from_str() {
        let id: RandomValue<4> = RandomValue::from_str("abcd").unwrap();
        assert_eq!(id.as_str(), "abcd", "end string is same as starting string");
    }

    #[test]
    fn source_length_and_definition_mismatch() {
        let str_id = "123";
        let id = RandomValue::<4>::from_bytes(str_id.as_bytes());
        assert!(id.is_err(), "length mismatch returns error value");
    }

    #[test]
    fn serialize_deserialize() {
        let str_id = "123";
        let id = RandomValue::<3>::from_bytes(str_id.as_bytes()).unwrap();
        let json_payload = serde_json::json!({
            "id": id,
        });
        let json_string = json_payload.to_string();
        assert_eq!(
            json_string, r#"{"id":"123"}"#,
            "Id is correctly serialized then deserialized"
        );
    }

    #[test]
    fn test_is_empty() {
        let id: RandomValue<4> = RandomValue::from_str("abcd").unwrap();
        assert!(id.len() > 0, "random value has a positive length");
        assert!(!id.is_empty(), "random value is NOT empty");

        let id: RandomValue<0> = RandomValue::from_str("").unwrap();
        assert!(id.len() == 0, "random value has ZERO length");
        assert!(id.is_empty(), "random value IS empty");
    }
}
