use std::fmt::Debug;
use std::collections::HashMap;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
pub enum Number {
    Int(i64),
    UInt(u64),
    Float(f64),
}

macro_rules! logical_operation {
    ($name:ident, $op:tt) => {
        pub fn $name(&self, other: &Number) -> bool {
            match (self, other) {
                (Number::Int(x), Number::Int(y)) => *x $op *y,
                (Number::Int(x), Number::UInt(y)) => (*x as i128) $op (*y as i128),
                (Number::Int(x), Number::Float(y)) => (*x as f64) $op *y,
                (Number::UInt(x), Number::Int(y)) => (*x as i128) $op (*y as i128),
                (Number::UInt(x), Number::UInt(y)) => *x $op *y,
                (Number::UInt(x), Number::Float(y)) => (*x as f64) $op *y,
                (Number::Float(x), Number::Int(y)) => *x $op (*y as f64),
                (Number::Float(x), Number::UInt(y)) => *x $op (*y as f64),
                (Number::Float(x), Number::Float(y)) => *x $op *y,
            }
        }
    };
}

macro_rules! arithmetic_operation {
    ($name:ident, $op:tt) => {
        pub fn $name(&self, other: &Number) -> Number {
            match (self, other) {
                (Number::Int(x), Number::Int(y)) => Number::Int(*x $op *y),
                (Number::Int(x), Number::UInt(y)) => Number::Int(*x $op (*y as i64)),
                (Number::Int(x), Number::Float(y)) => Number::Float((*x as f64) $op *y),
                (Number::UInt(x), Number::Int(y)) => Number::Int((*x as i64) $op *y),
                (Number::UInt(x), Number::UInt(y)) => Number::UInt(*x $op *y),
                (Number::UInt(x), Number::Float(y)) => Number::Float((*x as f64) $op *y),
                (Number::Float(x), Number::Int(y)) => Number::Float(*x $op (*y as f64)),
                (Number::Float(x), Number::UInt(y)) => Number::Float(*x $op (*y as f64)),
                (Number::Float(x), Number::Float(y)) => Number::Float(*x $op *y),
            }
        }
    };
}

impl Number {
    logical_operation!(eq, ==);
    logical_operation!(neq, !=);
    logical_operation!(lt, <);
    logical_operation!(lte, <=);
    logical_operation!(gt, >);
    logical_operation!(gte, >=);
    arithmetic_operation!(add, +);
    arithmetic_operation!(sub, -);
    arithmetic_operation!(mul, *);
    arithmetic_operation!(div, /);
    arithmetic_operation!(modulo, %);

    pub fn checked_div(&self, rhs: &Number) -> Option<Self> {
        match rhs {
            Number::Int(d) if *d == 0 => return None,
            Number::UInt(d) if *d == 0 => return None,
            Number::Float(d) if *d == 0.0 => return None,
            _ => {},
        }
        Some(self.div(rhs))
    }

    pub fn checked_modulo(&self, rhs: &Number) -> Option<Self> {
        match rhs {
            Number::Int(d) if *d == 0 => return None,
            Number::UInt(d) if *d == 0 => return None,
            Number::Float(d) if *d == 0.0 => return None,
            _ => {},
        }
        Some(self.modulo(rhs))
    }
}

pub trait Value: Debug {
    fn eq(&self, other: &dyn Value) -> bool {
        match (self.as_str(), other.as_str()) {
            (Some(s1), Some(s2)) if s1 == s2 => return true,
            _ => {},
        }
        match (self.as_int(), other.as_int()) {
            (Some(s1), Some(s2)) if s1 == s2 => return true,
            _ => {},
        }
        match (self.as_uint(), other.as_uint()) {
            (Some(s1), Some(s2)) if s1 == s2 => return true,
            _ => {},
        }
        match (self.as_float(), other.as_float()) {
            (Some(s1), Some(s2)) if s1 == s2 => return true,
            _ => {},
        }
        match (self.as_bool(), other.as_bool()) {
            (Some(s1), Some(s2)) if s1 == s2 => return true,
            _ => {},
        }

        // TODO: other types

        false
    }

    fn as_str(&self) -> Option<&str> { None }
    fn as_int(&self) -> Option<i64> { None }
    fn as_uint(&self) -> Option<u64> { None }
    fn as_float(&self) -> Option<f64> { None }
    fn as_bool(&self) -> Option<bool> { None }
    fn is_array(&self) -> bool { false }
    fn len(&self) -> Option<usize> { None }
    fn get(&self, index: usize) -> Option<&dyn Value> { None }
    fn is_object(&self) -> bool { false }
    fn get_prop(&self, prop: &str) -> Option<&dyn Value> { None }

    fn as_number(&self) -> Option<Number> {
        self.as_float().map(Number::Float)
            .or_else(|| self.as_int().map(Number::Int))
            .or_else(|| self.as_uint().map(Number::UInt))
    }

    fn to_number(&self) -> Option<f64> {
        self.as_float()
            .or_else(|| self.as_int().map(|v| v as f64))
            .or_else(|| self.as_uint().map(|v| v as f64))
    }

    fn get_by_key<'v>(&'v self, key: &str) -> Option<&'v dyn Value> {
        if self.is_array() {
            let index = key.parse().ok()?;
            self.get(index)
        } else if self.is_object() {
            self.get_prop(key)
        } else {
            None
        }
    }

    fn get_by_pointer<'a>(&'a self, pointer: &str) -> Option<&'a dyn Value> {
        let mut path = pointer.split('.');
        let mut result: &dyn Value = self.get_by_key(path.next()?)?;
        for p in path {
            result = result.get_by_key(p)?;
        }
        Some(result)
    }

    fn render(&self) -> Cow<str> {
        if let Some(s) = self.as_str() {
            Cow::Borrowed(s)
        } else if let Some(v) = self.as_int() {
            Cow::Owned(v.to_string())
        } else if let Some(v) = self.as_uint() {
            Cow::Owned(v.to_string())
        } else if let Some(v) = self.as_float() {
            Cow::Owned(v.to_string())
        } else if let Some(v) = self.as_bool() {
            Cow::Owned(v.to_string())
        } else if self.is_array() {
            let mut buf = String::new();
            buf.push('[');
            for i in 0..self.len().unwrap() {
                if buf.len() > 1 {
                    buf.push_str(", ");
                }
                buf.push_str(self.get(i).unwrap().render().as_ref());
            }
            buf.push(']');
            Cow::Owned(buf)
        } else if self.is_object() {
            Cow::Borrowed("[object]")
        } else {
            Cow::Borrowed("")
        }
    }
}

impl Value for &str {
    fn as_str(&self) -> Option<&str> { Some(self) }
}

impl Value for String {
    fn as_str(&self) -> Option<&str> { Some(self) }
}

impl Value for i32 {
    fn as_int(&self) -> Option<i64> { Some(*self as i64) }
}

impl Value for i64 {
    fn as_int(&self) -> Option<i64> { Some(*self) }
}

impl Value for u64 {
    fn as_uint(&self) -> Option<u64> { Some(*self) }
}

impl Value for f64 {
    fn as_float(&self) -> Option<f64> { Some(*self) }
}

impl Value for usize {
    fn as_uint(&self) -> Option<u64> { Some(*self as u64) }
}

impl Value for bool {
    fn as_bool(&self) -> Option<bool> { Some(*self) }
}

impl Value for HashMap<String, Box<dyn Value>> {
    fn is_object(&self) -> bool { true }
    fn get_prop(&self, prop: &str) -> Option<&dyn Value> { self.get(prop).map(|v| &**v as &Value) }
}

impl<T: Value> Value for Vec<T> {
    fn is_array(&self) -> bool { true }
    fn len(&self) -> Option<usize> { Some(self.len()) }
    fn get(&self, index: usize) -> Option<&dyn Value> { self.get(index).map(|v| &*v as &Value) }
}

impl Value for Vec<&dyn Value> {
    fn is_array(&self) -> bool { true }
    fn len(&self) -> Option<usize> { Some(self.len()) }
    fn get(&self, index: usize) -> Option<&dyn Value> { self.get(index).map(|v| &*v as &Value) }
}

impl Value for serde_json::value::Value {
    fn as_str(&self) -> Option<&str> { self.as_str() }
    fn as_int(&self) -> Option<i64> { self.as_i64() }
    fn as_uint(&self) -> Option<u64> { self.as_u64() }
    fn as_float(&self) -> Option<f64> { self.as_f64() }
    fn as_bool(&self) -> Option<bool> { self.as_bool() }
    fn is_array(&self) -> bool { self.is_array() }
    fn len(&self) -> Option<usize> { self.as_array().map(|a| a.len()) }
    fn get(&self, index: usize) -> Option<&dyn Value> { self.get(index).map(|v| &*v as &Value) }
    fn is_object(&self) -> bool { self.is_object() }
    fn get_prop(&self, prop: &str) -> Option<&dyn Value> { self.get(prop).map(|v| &*v as &Value) }
}

pub enum ValueRef<'t> {
    Ref(&'t dyn Value),
    Val(Box<dyn Value>),
}

impl<'t> ValueRef<'t> {
    pub fn borrowed(value: &'t dyn Value) -> Self {
        ValueRef::Ref(value)
    }

    pub fn owned(value: impl Value + 'static) -> Self {
        ValueRef::Val(Box::new(value))
    }
}

impl<'t> std::ops::Deref for ValueRef<'t> {
    type Target = dyn Value + 't;

    fn deref(&self) -> &Self::Target {
        match self {
            ValueRef::Ref(r) => *r,
            ValueRef::Val(b) => &**b,
        }
    }
}
