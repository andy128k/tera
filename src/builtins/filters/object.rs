/// Filters operating on numbers
use std::collections::HashMap;

use crate::errors::{Error, Result};
use crate::value::Value;

/// Returns a value by a `key` argument from a given object
pub fn get(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let key = match args.get("key") {
        Some(val) => val.try_str().map_err(|e| Error::chain("`key` argument", e))?,
        None => return Err(Error::msg("The `get` filter has to have an `key` argument")),
    };

    value.try_object()?.get(key).cloned().ok_or_else(|| {
        Error::msg(format!("Filter `get` tried to get key `{}` but it wasn't found", &key))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::to_value;
    use std::collections::HashMap;

    #[test]
    fn test_get_filter_exists() {
        let obj = Value::Object({
            let mut obj = HashMap::new();
            obj.insert("1".to_string(), Value::String("first".to_string()));
            obj.insert("2".to_string(), Value::String("second".to_string()));
            obj
        });

        let mut args = HashMap::new();
        args.insert("key".to_string(), Value::String("1".to_string()));
        let result = get(&obj, &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("first".to_string()));
    }

    #[test]
    fn test_get_filter_doesnt_exist() {
        let obj = Value::Object({
            let mut obj = HashMap::new();
            obj.insert("1".to_string(), Value::String("first".to_string()));
            obj.insert("2".to_string(), Value::String("second".to_string()));
            obj
        });

        let mut args = HashMap::new();
        args.insert("key".to_string(), Value::String("3".to_string()));
        let result = get(&obj, &args);
        assert!(result.is_err());
    }
}
