#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]

use indexmap::IndexMap;
use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};
use serde_json::Value;
use std::fmt;

/// OrderedJson is similar to Go's orderedjson.Object - it maintains insertion order of keys
#[derive(Debug, Clone, PartialEq)]
pub struct OrderedJson {
    pub(crate) map: IndexMap<String, Value>,
}

impl OrderedJson {
    pub fn new() -> Self {
        OrderedJson {
            map: IndexMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        OrderedJson {
            map: IndexMap::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.map.get(key)
    }

    pub fn insert(&mut self, key: String, value: Value) -> Option<Value> {
        self.map.insert(key, value)
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.map.shift_remove(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.map.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.map.values()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.map.iter()
    }
}

impl Default for OrderedJson {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for OrderedJson {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.map.len()))?;
        for (k, v) in &self.map {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for OrderedJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OrderedJsonVisitor;

        impl<'de> Visitor<'de> for OrderedJsonVisitor {
            type Value = OrderedJson;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut map = IndexMap::with_capacity(access.size_hint().unwrap_or(0));

                while let Some((key, value)) = access.next_entry::<String, Value>()? {
                    map.insert(key, value);
                }

                Ok(OrderedJson { map })
            }
        }

        deserializer.deserialize_map(OrderedJsonVisitor)
    }
}

impl From<IndexMap<String, Value>> for OrderedJson {
    fn from(map: IndexMap<String, Value>) -> Self {
        OrderedJson { map }
    }
}

impl From<OrderedJson> for IndexMap<String, Value> {
    fn from(obj: OrderedJson) -> Self {
        obj.map
    }
}

impl FromIterator<(String, Value)> for OrderedJson {
    fn from_iter<I: IntoIterator<Item = (String, Value)>>(iter: I) -> Self {
        OrderedJson {
            map: IndexMap::from_iter(iter),
        }
    }
}
