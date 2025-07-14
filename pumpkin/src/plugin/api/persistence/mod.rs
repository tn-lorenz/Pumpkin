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
            log::error!("Invalid namespace: '{namespace}' is not pure ASCII.");
            return Err(NamespacedKeyError::NonAsciiNamespace);
        }
        if !key.is_ascii() {
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
            Err(e) => panic!("Invalid key: {:?}", e),
        }
    };
}

/// The `PersistentDataContainer` struct
///
/// This struct contains `NamespacedKey`s and associates them with `PersistentValue`s using a `DashMap` for maximum concurrency.
#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct PersistentDataContainer {
    pub data: DashMap<NamespacedKey, PersistentValue>,
}

#[allow(dead_code)]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum PersistentValue {
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
    List(Vec<PersistentValue>),
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
    pub fn get(&self, key: &NamespacedKey) -> Option<PersistentValue> {
        self.data.get(key).map(|v| v.clone())
    }

    pub fn insert(&self, key: &NamespacedKey, value: PersistentValue) {
        self.data.insert(key.clone(), value);
    }

    #[must_use]
    pub fn remove(&self, key: &NamespacedKey) -> Option<(NamespacedKey, PersistentValue)> {
        self.data.remove(key)
    }

    #[must_use]
    pub fn contains_key(&self, key: &NamespacedKey) -> bool {
        self.data.contains_key(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (NamespacedKey, PersistentValue)> + '_ {
        self.data
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
    }
}

pub trait HasPersistentContainer {
    fn persistent_container(&self) -> &PersistentDataContainer;
}

pub trait PersistentDataHolder {
    fn clear(&self);
    fn get(&self, key: &NamespacedKey) -> Option<PersistentValue>;
    fn insert(&self, key: &NamespacedKey, value: PersistentValue);
    fn remove(&self, key: &NamespacedKey) -> Option<PersistentValue>;
    fn contains_key(&self, key: &NamespacedKey) -> bool;
    fn iter(&self) -> Box<dyn Iterator<Item = (NamespacedKey, PersistentValue)> + '_>;
}

impl<T: HasPersistentContainer> PersistentDataHolder for T {
    fn clear(&self) {
        self.persistent_container().clear();
    }

    fn get(&self, key: &NamespacedKey) -> Option<PersistentValue> {
        self.persistent_container().get(key)
    }

    fn insert(&self, key: &NamespacedKey, value: PersistentValue) {
        self.persistent_container().insert(key, value);
    }

    fn remove(&self, key: &NamespacedKey) -> Option<PersistentValue> {
        self.persistent_container().remove(key).map(|(_, v)| v)
    }

    fn contains_key(&self, key: &NamespacedKey) -> bool {
        self.persistent_container().contains_key(key)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (NamespacedKey, PersistentValue)> + '_> {
        Box::new(self.persistent_container().iter())
    }
}
