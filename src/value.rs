use std::collections::HashMap;
use crate::errors::{Error, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    pub fn empty_string() -> Self {
        Value::String("".to_owned())
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Integer(i) => *i != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            Value::Bool(i) => *i,
            Value::String(ref i) => !i.is_empty(),
            Value::Array(ref i) => !i.is_empty(),
            Value::Object(ref i) => !i.is_empty(),
        }
    }

    pub fn try_integer(&self) -> Result<i64> {
        match self {
            Value::Integer(i) => Ok(*i),
            val => Err(Error::msg(format!("expected integer number got {:?}", val))),
        }
    }

    pub fn try_float(&self) -> Result<f64> {
        match self {
            Value::Float(f) => Ok(*f),
            val => Err(Error::msg(format!("expected float number got {:?}", val))),
        }
    }

    pub fn try_bool(&self) -> Result<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            val => Err(Error::msg(format!("expected boolean got {:?}", val))),
        }
    }

    pub fn try_str(&self) -> Result<&str> {
        match self {
            Value::String(ref s) => Ok(s),
            val => Err(Error::msg(format!("expected string got {:?}", val))),
        }
    }

    pub fn try_array(&self) -> Result<&[Value]> {
        match self {
            Value::Array(ref a) => Ok(a),
            val => Err(Error::msg(format!("expected array got {:?}", val))),
        }
    }

    pub fn try_object(&self) -> Result<&HashMap<String, Value>> {
        match self {
            Value::Object(ref o) => Ok(o),
            val => Err(Error::msg(format!("expected object got {:?}", val))),
        }
    }

    pub fn to_number(&self) -> std::result::Result<f64, ()> {
        match self {
            Value::Integer(i) => Ok(*i as f64),
            Value::Float(f) => Ok(*f),
            _ => Err(()),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(ref obj) => obj.get(key),
            _ => None,
        }
    }

    fn get_by_key<'v>(&'v self, key: &str) -> Option<&'v Value> {
        match self {
            Value::Object(ref obj) => obj.get(key),
            Value::Array(ref arr) => {
                let index: usize = key.parse().ok()?;
                arr.get(index)
            },
            _ => None,
        }
    }

    pub fn pointer<'v>(&'v self, pointer: &str) -> Option<&'v Value> {
        let mut path = pointer.split('.');
        let mut result: &Value = self.get_by_key(path.next()?)?;
        for p in path {
            result = result.get_by_key(p)?;
        }
        Some(result)
    }
}

impl std::convert::Into<serde_json::Value> for Value {
    fn into(self) -> serde_json::Value {
        match self {
            Value::Bool(b) => serde_json::Value::Bool(b),
            Value::Integer(i) => serde_json::Value::Number(i.into()),
            Value::Float(f) => serde_json::Number::from_f64(f).map_or(serde_json::Value::Null, serde_json::Value::Number),
            Value::String(s) => serde_json::Value::String(s),
            Value::Array(v) => serde_json::Value::Array(v.into_iter().map(Into::into).collect()),
            Value::Object(m) => serde_json::Value::Object(m.into_iter().fold(serde_json::Map::new(), |map, (k, v)| { map.insert(k, v.into()); map })),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Array(ref v) => {
                write!(f, "[")?;
                let mut first = true;
                for i in v {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", i)?;
                    first = false;
                }
                write!(f, "]")
            },
            Value::Object(ref m) => {
                write!(f, "{{")?;
                let mut first = true;
                for (k, v) in m {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                    first = false;
                }
                write!(f, "}}")
            },
        }
    }
}
