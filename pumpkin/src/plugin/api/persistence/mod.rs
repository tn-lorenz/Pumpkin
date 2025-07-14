use dashmap::DashMap;
use serde::{Deserialize, Serialize};

/// The `NamespacedKey` struct
#[allow(dead_code)]
#[derive(Eq, Hash, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct NamespacedKey {
    namespace: String,
    key: String,
}

#[derive(Debug)]
pub enum NamespacedKeyError {
    NonAsciiNamespace,
    NonAsciiKey,
}

/// The `NamespacedKey` constructor
///
/// `new()` must only be called via the `ns_key!` macro.
///
/// # Parameters
/// - `namespace`: namespace of the key, must be equal to the `CARGO_PKG_NAME`
/// - `key`: The key as a String
///
/// # Returns
/// - Self
impl NamespacedKey {
    #[allow(dead_code)]
    pub(crate) fn new(namespace: &str, key: &str) -> Result<Self, NamespacedKeyError> {
        if !namespace.is_ascii() {
            #[cfg(debug_assertions)]
            log::error!("Invalid namespace: '{namespace}' is not pure ASCII.");
            return Err(NamespacedKeyError::NonAsciiNamespace);
        }
        if !key.is_ascii() {
            #[cfg(debug_assertions)]
            log::error!("Invalid key: '{key}' is not pure ASCII.");
            return Err(NamespacedKeyError::NonAsciiKey);
        }

        Ok(Self {
            namespace: namespace.to_ascii_lowercase(),
            key: key.to_ascii_lowercase(),
        })
    }
}

/// A macro used to create a new `NamespacedKey` without having to manually pass the `CARGO_PKG_NAME` by using the `env!()` macro
///
/// # Parameters
/// - `$value`: the key you want to create as a String.
#[macro_export]
macro_rules! ns_key {
    ($value:expr) => {
        match $crate::plugin::NamespacedKey::new(env!("CARGO_PKG_NAME"), $value) {
            Ok(key) => key,
            Err(e) => panic!("ns_key! macro failed: {:?}", e);
        }
    };
}

/// The `PersistentDataContainer` struct
///
/// This struct contains `NamespacedKey`s and associates them with `PersistentValue`s using a `DashMap` for maximum concurrency.
#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct PersistentDataContainer {
    pub data: DashMap<NamespacedKey, PersistentDataType>,
}

/// A list of all currently allowed Types that can be stored inside a `PersistentDataContainer`
#[allow(dead_code)]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum PersistentDataType {
    Bool(bool),
    String(String),
    Char(char),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Bytes(Vec<u8>),
    List(Vec<PersistentDataType>),
}

impl PersistentDataContainer {
    pub(crate) fn new() -> Self {
        Self {
            data: DashMap::new(),
        }
    }

    pub fn clear(&self) {
        self.data.clear();
    }

    #[must_use]
    pub fn get(&self, key: &NamespacedKey) -> Option<PersistentDataType> {
        self.data.get(key).map(|v| v.clone())
    }

    #[must_use]
    pub fn get_as<T: FromPersistentDataType>(&self, key: &NamespacedKey) -> Option<T> {
        self.get(key).and_then(|v| T::from_persistent(&v))
    }

    pub fn insert(&self, key: &NamespacedKey, value: PersistentDataType) {
        self.data.insert(key.clone(), value);
    }

    #[must_use]
    pub fn remove(&self, key: &NamespacedKey) -> Option<(NamespacedKey, PersistentDataType)> {
        self.data.remove(key)
    }

    #[must_use]
    pub fn contains_key(&self, key: &NamespacedKey) -> bool {
        self.data.contains_key(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (NamespacedKey, PersistentDataType)> + '_ {
        self.data
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
    }
}

/// A trait that defines functions on a struct that holds a `PersistentDataContainer`
///
/// These are used inside the `PersistentDataHolder` derive macro that defines a struct as a holder of a `PersistentDataContainer` by generating the implementation of these functions.
pub trait PersistentDataHolder {
    fn clear(&self);
    fn get(&self, key: &NamespacedKey) -> Option<PersistentDataType>;
    fn get_as<T: FromPersistentDataType>(&self, key: &NamespacedKey) -> Option<T>;
    fn insert(&self, key: &NamespacedKey, value: PersistentDataType);
    fn remove(&self, key: &NamespacedKey) -> Option<PersistentDataType>;
    fn contains_key(&self, key: &NamespacedKey) -> bool;
    fn iter(&self) -> Box<dyn Iterator<Item = (NamespacedKey, PersistentDataType)> + '_>;
}

/// Gets the actual value that has been wrapped inside a `PersistentDataType`
pub trait FromPersistentDataType: Sized {
    fn from_persistent(value: &PersistentDataType) -> Option<Self>;
}

/// This simple proc macro enables easy implementation of the `FromPersistentDataType` trait because the logic is trivial
#[macro_export]
macro_rules! from_persistent {
    // Copy types
    ($variant:ident, $ty:ty) => {
        impl FromPersistentDataType for $ty {
            fn from_persistent(value: &PersistentDataType) -> Option<Self> {
                match value {
                    PersistentDataType::$variant(v) => Some(*v),
                    _ => None,
                }
            }
        }
    };

    // Clone types
    (clone $variant:ident, $ty:ty) => {
        impl FromPersistentDataType for $ty {
            fn from_persistent(value: &PersistentDataType) -> Option<Self> {
                match value {
                    PersistentDataType::$variant(v) => Some(v.clone()),
                    _ => None,
                }
            }
        }
    };
}

// Copy types
from_persistent!(Bool, bool);
from_persistent!(Char, char);
from_persistent!(I32, i32);
from_persistent!(I64, i64);
from_persistent!(U8, u8);
from_persistent!(U16, u16);
from_persistent!(U32, u32);
from_persistent!(U64, u64);
from_persistent!(F32, f32);
from_persistent!(F64, f64);

// Clone types
from_persistent!(clone String, String);
from_persistent!(clone Bytes, Vec<u8>);
from_persistent!(clone List, Vec<PersistentDataType>);
