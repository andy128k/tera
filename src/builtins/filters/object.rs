/// Filters operating on numbers
use std::collections::HashMap;

use value::{Value, ValueRef};
use errors::{Error, Result};

/// Returns a value by a `key` argument from a given object
pub fn get<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let key = args.get("key").ok_or_else(|| Error::msg("The `get` filter has to have an `key` argument"))?;
    let key = key.as_str().ok_or_else(|| {
        Error::msg(format!("Filter `get` received an incorrect type for arg `key`: got `{:?}` but expected a String", key))
    })?;

    if !value.is_object() {
        return Err(Error::msg("Filter `get` was used on a value that isn't an object"));
    }

    let prop = value.get_prop(key).ok_or_else(|| {
        Error::msg(format!("Filter `get` tried to get key `{}` but it wasn't found", &key))
    })?;

    Ok(ValueRef::borrowed(prop))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::to_value;
    use std::collections::HashMap;

    #[test]
    fn test_get_filter_exists() {
        let mut obj = HashMap::new();
        obj.insert("1".to_string(), "first".to_string());
        obj.insert("2".to_string(), "second".to_string());

        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("key".to_string(), Box::new("1"));
        let result = get(&to_value(&obj).unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"first"));
    }

    #[test]
    fn test_get_filter_doesnt_exist() {
        let mut obj = HashMap::new();
        obj.insert("1".to_string(), "first".to_string());
        obj.insert("2".to_string(), "second".to_string());

        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("key".to_string(), Box::new("3"));
        let result = get(&to_value(&obj).unwrap(), &args);
        assert!(result.is_err());
    }
}
