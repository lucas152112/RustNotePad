use std::borrow::Cow;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::de::{Error as DeError, Unexpected, Visitor};
use serde::{Deserializer, Serializer};

const B64_PREFIX: &str = "b64:";

/// Serialises a `Path` into a string, keeping UTF-8 intact when possible and
/// falling back to base64 when necessary.  
/// 若路徑為 UTF-8 字串則直接輸出；否則以 base64 保存。
pub fn serialize<S>(path: &Path, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let encoded = encode(path);
    serializer.serialize_str(&encoded)
}

/// Deserialises a `PathBuf` from a string produced by [`serialize`].  
/// 從上述序列化結果還原 `PathBuf`。
pub fn deserialize<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    struct PathVisitor;

    impl<'de> Visitor<'de> for PathVisitor {
        type Value = PathBuf;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a UTF-8 or base64 encoded path string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            decode(v).map_err(E::custom)
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            decode(&v).map_err(E::custom)
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            let value = std::str::from_utf8(v)
                .map_err(|_| E::invalid_value(Unexpected::Bytes(v), &"a UTF-8/base64 path"))?;
            decode(value).map_err(E::custom)
        }
    }

    deserializer.deserialize_any(PathVisitor)
}

/// Serde helpers for `Option<PathBuf>`.
pub mod option {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(path) => serializer.serialize_some(&encode(path)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            Some(text) => decode(&text).map(Some).map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

fn encode(path: &Path) -> String {
    match path.to_str() {
        Some(text) => text.to_string(),
        None => {
            let bytes = path_to_bytes(path);
            let b64 = BASE64.encode(bytes);
            format!("{B64_PREFIX}{b64}")
        }
    }
}

fn decode(text: &str) -> Result<PathBuf, String> {
    if let Some(rest) = text.strip_prefix(B64_PREFIX) {
        let bytes = BASE64
            .decode(rest.as_bytes())
            .map_err(|err| format!("invalid base64 path payload: {err}"))?;
        bytes_to_path(bytes).map_err(|err| format!("invalid path payload: {err}"))
    } else {
        Ok(PathBuf::from(text))
    }
}

fn path_to_bytes(path: &Path) -> Cow<'_, [u8]> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Cow::Borrowed(path.as_os_str().as_bytes())
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        let mut bytes = Vec::with_capacity(wide.len() * 2);
        for unit in wide {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        Cow::Owned(bytes)
    }
}

fn bytes_to_path(bytes: Vec<u8>) -> Result<PathBuf, String> {
    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        Ok(PathBuf::from(OsString::from_vec(bytes)))
    }

    #[cfg(windows)]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        if bytes.len() % 2 != 0 {
            return Err("encoded Windows path has odd byte length".to_string());
        }
        let wide: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        Ok(PathBuf::from(OsString::from_wide(&wide)))
    }
}
