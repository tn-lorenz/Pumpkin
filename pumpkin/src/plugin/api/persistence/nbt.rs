use crate::plugin::persistence::{NamespacedKey, PersistentDataContainer, PersistentDataType};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_nbt::tag::NbtTag;

pub trait NbtCompoundExt {
    fn to_pdc(&self) -> PersistentDataContainer;
}

impl NbtCompoundExt for NbtCompound {
    fn to_pdc(&self) -> PersistentDataContainer {
        let container = PersistentDataContainer::new();

        for (namespace, ns_tag) in &self.child_tags {
            if let NbtTag::Compound(ns_compound) = ns_tag {
                for (key, tag) in &ns_compound.child_tags {
                    if let Ok(ns_key) = NamespacedKey::new(namespace, key) {
                        let value = match tag {
                            NbtTag::Byte(b) => PersistentDataType::Bool(*b != 0),
                            NbtTag::Short(s) => PersistentDataType::I32(i32::from(*s)),
                            NbtTag::Int(i) => PersistentDataType::I32(*i),
                            NbtTag::Long(l) => PersistentDataType::I64(*l),
                            NbtTag::Float(f) => PersistentDataType::F32(*f),
                            NbtTag::Double(d) => PersistentDataType::F64(*d),
                            NbtTag::String(s) => PersistentDataType::String(s.clone()),
                            NbtTag::ByteArray(bytes) => PersistentDataType::Bytes(bytes.clone()),
                            NbtTag::List(list) => PersistentDataType::List(
                                list.iter()
                                    .filter_map(|t| match t {
                                        NbtTag::Int(i) => Some(PersistentDataType::I32(*i)),
                                        NbtTag::String(s) => {
                                            Some(PersistentDataType::String(s.clone()))
                                        }
                                        _ => None,
                                    })
                                    .collect(),
                            ),
                            _ => continue, // Unsupported tag
                        };
                        container.insert(ns_key, value);
                    }
                }
            }
        }
        container
    }
}
