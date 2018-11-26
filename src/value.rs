pub trait Value {
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

        // TODO: other types

        false
    }

    fn as_str(&self) -> Option<&str>;
    fn as_int(&self) -> Option<i64>;
    fn as_uint(&self) -> Option<u64>;
    fn as_float(&self) -> Option<f64>;

    fn is_array(&self) -> bool;
    fn len(&self) -> Option<usize>;
    fn get(&self, index: usize) -> Option<&dyn Value>;

    fn is_object(&self) -> bool;
    fn get_prop(&self, prop: &str) -> Option<&dyn Value>;

    fn to_number(&self) -> Option<f64> {
        self.as_float()
            .or_else(|| self.as_int().map(|v| v as f64))
            .or_else(|| self.as_uint().map(|v| v as f64))
    }
}

impl Value for str {
    fn eq(&self, other: &dyn Value) -> bool { other.as_str().map_or(false, |o| o == self) }
    fn as_str(&self) -> Option<&str> { Some(self) }
    fn as_int(&self) -> Option<i64> { None }
    fn as_uint(&self) -> Option<u64> { None }
    fn as_float(&self) -> Option<f64> { None }
    fn is_array(&self) -> bool { false }
    fn len(&self) -> Option<usize> { None }
    fn get(&self, index: usize) -> Option<&dyn Value> { None }
    fn is_object(&self) -> bool { false }
    fn get_prop(&self, prop: &str) -> Option<&dyn Value> { None }
}

impl Value for serde_json::value::Value {
    fn as_str(&self) -> Option<&str> { self.as_str() }
    fn as_int(&self) -> Option<i64> { self.as_i64() }
    fn as_uint(&self) -> Option<u64> { self.as_u64() }
    fn as_float(&self) -> Option<f64> { self.as_f64() }
    fn is_array(&self) -> bool { self.is_array() }
    fn len(&self) -> Option<usize> { self.as_array().map(|a| a.len()) }
    fn get(&self, index: usize) -> Option<&dyn Value> { self.get(index).map(|v| &*v as &Value) }
    fn is_object(&self) -> bool { self.is_object() }
    fn get_prop(&self, prop: &str) -> Option<&dyn Value> { self.get(prop).map(|v| &*v as &Value) }
}
