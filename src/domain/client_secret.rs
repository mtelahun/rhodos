use serde::{Deserialize, Serialize};

use super::random_value::RandomValue;

const SECRETLEN: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientSecret {
    inner: RandomValue<SECRETLEN>,
}

impl ClientSecret {
    pub fn new() -> Self {
        Self {
            inner: nanoid::nanoid!(SECRETLEN).parse().unwrap(),
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

impl std::default::Default for ClientSecret {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientSecret, SECRETLEN};

    #[test]
    fn client_id_length() {
        let id = ClientSecret::new();
        assert!(!id.is_empty(), "secret is NOT empty");
        assert_eq!(
            id.len(),
            SECRETLEN,
            "length of client secret is {SECRETLEN}"
        );

        let str_id = id.as_str();
        assert_eq!(
            str_id.len(),
            SECRETLEN,
            "length of client secret string is {SECRETLEN}"
        );
    }
}
