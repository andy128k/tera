use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::ser::Serialize;
use serde::ser::SerializeMap;
use serde::Serializer;
use crate::value::Value;

/// The struct that holds the context of a template rendering.
///
/// Light wrapper around a `BTreeMap` for easier insertions of Serializable
/// values
#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    data: BTreeMap<String, Value>,
}

impl Context {
    /// Initializes an empty context
    pub fn new() -> Context {
        Context { data: BTreeMap::new() }
    }

    /// Converts the `val` parameter to `Value` and insert it into the context
    ///
    /// ```rust,ignore
    /// let mut context = Context::new();
    /// // user is an instance of a struct implementing `Serialize`
    /// context.insert("number_users", 42);
    /// ```
    pub fn insert<T: Serialize + ?Sized>(&mut self, key: &str, val: &T) {
        self.data.insert(key.to_owned(), to_value(val).unwrap());
    }

    /// Appends the data of the `source` parameter to `self`, overwriting existing keys.
    /// The source context will be dropped.
    ///
    /// ```rust,ignore
    /// let mut target = Context::new();
    /// target.insert("a", 1);
    /// target.insert("b", 2);
    /// let mut source = Context::new();
    /// source.insert("b", 3);
    /// source.insert("d", 4);
    /// target.extend(source);
    /// ```
    pub fn extend(&mut self, mut source: Context) {
        self.data.append(&mut source.data);
    }
}

impl Default for Context {
    fn default() -> Context {
        Context::new()
    }
}

impl Serialize for Context {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.data.len()))?;
        for (k, v) in &self.data {
            map.serialize_key(&k)?;
            map.serialize_value(&v)?;
        }
        map.end()
    }
}

pub trait ValueRender {
    fn render(&self) -> Cow<str>;
}

// Convert serde Value to String
impl ValueRender for Value {
    fn render(&self) -> Cow<str> {
        match *self {
            Value::String(ref s) => Cow::Borrowed(s),
            Value::Integer(i) => Cow::Owned(i.to_string()),
            Value::Float(f) => Cow::Owned(f.to_string()),
            Value::Bool(i) => Cow::Owned(i.to_string()),
            Value::Array(ref a) => {
                let mut buf = String::new();
                buf.push('[');
                for i in a.iter() {
                    if buf.len() > 1 {
                        buf.push_str(", ");
                    }
                    buf.push_str(i.render().as_ref());
                }
                buf.push(']');
                Cow::Owned(buf)
            }
            Value::Object(_) => Cow::Owned("[object]".to_owned()),
        }
    }
}

/// Converts a dotted path to a json pointer one
#[inline]
pub fn get_json_pointer(key: &str) -> String {
    ["/", &key.replace(".", "/")].join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extend() {
        let mut target = Context::new();
        target.insert("a", &1);
        target.insert("b", &2);
        let mut source = Context::new();
        source.insert("b", &3);
        source.insert("c", &4);
        target.extend(source);
        assert_eq!(*target.data.get("a").unwrap(), Value::Integer(1));
        assert_eq!(*target.data.get("b").unwrap(), Value::Integer(3));
        assert_eq!(*target.data.get("c").unwrap(), Value::Integer(4));
    }
}
