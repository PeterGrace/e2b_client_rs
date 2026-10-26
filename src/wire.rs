//! Serde helpers for the protobuf JSON mapping envd speaks.
//!
//! The official SDKs talk to envd with Connect's JSON codec
//! (`e2b/envd/client_shared.py:29-60`; JS `useBinaryFormat: false`,
//! `dist/index.js:5824-5827`), so messages travel as proto3 canonical JSON:
//! `bytes` fields are base64 strings, `int64` fields are decimal strings (but
//! parsers must also accept numbers), and enums are their value names (parsers
//! must also accept the numeric value).

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::{
    STANDARD, STANDARD_PAD_INDIFFERENT, URL_SAFE_PAD_INDIFFERENT,
};
use bytes::Bytes;
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serializer};

/// Serializes bytes as a padded standard-alphabet base64 string.
pub(crate) fn serialize_base64<S: Serializer>(
    data: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&STANDARD.encode(data))
}

/// Deserializes an optional proto3 JSON `bytes` value. Proto3 JSON parsers
/// accept both the standard and URL-safe alphabets, with or without padding.
pub(crate) fn deserialize_base64_opt<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Bytes>, D::Error> {
    let encoded = Option::<String>::deserialize(deserializer)?;
    encoded
        .map(|value| decode_base64(&value).map_err(de::Error::custom))
        .transpose()
}

fn decode_base64(encoded: &str) -> Result<Bytes, base64::DecodeError> {
    STANDARD_PAD_INDIFFERENT
        .decode(encoded)
        .or_else(|_| URL_SAFE_PAD_INDIFFERENT.decode(encoded))
        .map(Bytes::from)
}

/// Deserializes a proto3 JSON `int64`, which may be a string or a number.
pub(crate) fn deserialize_i64_lenient<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<i64, D::Error> {
    struct I64Visitor;

    impl Visitor<'_> for I64Visitor {
        type Value = i64;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an int64 as a JSON number or decimal string")
        }

        fn visit_i64<E: de::Error>(self, value: i64) -> Result<i64, E> {
            Ok(value)
        }

        fn visit_u64<E: de::Error>(self, value: u64) -> Result<i64, E> {
            i64::try_from(value).map_err(E::custom)
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<i64, E> {
            value.parse().map_err(E::custom)
        }
    }

    deserializer.deserialize_any(I64Visitor)
}

/// A proto3 JSON enum value: either its name or its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EnumValue {
    /// The value name, e.g. `"FILE_TYPE_FILE"`.
    Name(String),
    /// The numeric value, e.g. `1`.
    Number(i64),
}

impl<'de> Deserialize<'de> for EnumValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EnumVisitor;

        impl Visitor<'_> for EnumVisitor {
            type Value = EnumValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an enum value name or number")
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<EnumValue, E> {
                Ok(EnumValue::Number(value))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<EnumValue, E> {
                i64::try_from(value)
                    .map(EnumValue::Number)
                    .map_err(E::custom)
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<EnumValue, E> {
                Ok(EnumValue::Name(value.to_owned()))
            }
        }

        deserializer.deserialize_any(EnumVisitor)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct BytesHolder {
        #[serde(
            serialize_with = "serialize_opt",
            deserialize_with = "deserialize_base64_opt"
        )]
        data: Option<Bytes>,
    }

    fn serialize_opt<S: Serializer>(
        data: &Option<Bytes>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serialize_base64(data.as_deref().unwrap_or_default(), serializer)
    }

    #[derive(Deserialize)]
    struct IntHolder {
        #[serde(deserialize_with = "deserialize_i64_lenient")]
        size: i64,
    }

    #[test]
    fn base64_round_trip() {
        let holder = BytesHolder {
            data: Some(Bytes::from_static(b"hello\xff")),
        };
        let json = serde_json::to_string(&holder).expect("serialize");
        assert_eq!(json, r#"{"data":"aGVsbG//"}"#);
        let back: BytesHolder = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.data.as_deref(), Some(&b"hello\xff"[..]));
    }

    #[test]
    fn base64_accepts_url_safe_and_unpadded() {
        let back: BytesHolder = serde_json::from_str(r#"{"data":"aGVsbG__"}"#).expect("url-safe");
        assert_eq!(back.data.as_deref(), Some(&b"hello\xff"[..]));
        let back: BytesHolder = serde_json::from_str(r#"{"data":"aGk"}"#).expect("unpadded");
        assert_eq!(back.data.as_deref(), Some(&b"hi"[..]));
    }

    #[test]
    fn base64_rejects_garbage() {
        assert!(serde_json::from_str::<BytesHolder>(r#"{"data":"!!!"}"#).is_err());
    }

    #[test]
    fn int64_accepts_string_and_number() {
        let from_str: IntHolder = serde_json::from_str(r#"{"size":"42"}"#).expect("string");
        let from_num: IntHolder = serde_json::from_str(r#"{"size":42}"#).expect("number");
        assert_eq!(from_str.size, 42);
        assert_eq!(from_num.size, 42);
        assert!(serde_json::from_str::<IntHolder>(r#"{"size":"x"}"#).is_err());
    }

    #[test]
    fn enum_value_accepts_name_and_number() {
        let name: EnumValue = serde_json::from_str(r#""FILE_TYPE_FILE""#).expect("name");
        let number: EnumValue = serde_json::from_str("2").expect("number");
        assert_eq!(name, EnumValue::Name("FILE_TYPE_FILE".into()));
        assert_eq!(number, EnumValue::Number(2));
    }
}
