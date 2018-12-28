/// Filters operating on numbers
use std::collections::HashMap;

use humansize::{file_size_opts, FileSize};

use crate::errors::{Error, Result};
use crate::value::Value;

/// Returns a suffix if the value is not equal to ±1. Suffix defaults to `s`
pub fn pluralize(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    // English uses plural when it isn't one
    let is_plural = match value {
        Value::Integer(num) => num.abs() == 1,
        Value::Float(num) => (num.abs() - 1.).abs() > ::std::f64::EPSILON,
        val => return Err(Error::msg(format!("expected number got {:?}", val))),
    };

    if is_plural {
        let suffix = match args.get("suffix") {
            Some(val) => val.try_str().map_err(|e| Error::chain("suffix argument", e))?,
            None => "s",
        };
        Ok(Value::String(suffix.to_string()))
    } else {
        Ok(Value::String("".to_string()))
    }
}

/// Returns a rounded number using the `method` arg and `precision` given.
/// `method` defaults to `common` which will round to the nearest number.
/// `ceil` and `floor` are also available as method.
/// `precision` defaults to `0`, meaning it will round to an integer
pub fn round(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let num = match value {
        Value::Integer(num) => return Ok(value.clone()),
        Value::Float(num) => num,
        val => return Err(Error::msg(format!("expected number got {:?}", val))),
    };
    let method = match args.get("method") {
        Some(val) => val.try_str().map_err(|e| Error::chain("`method` argument", e))?,
        None => "common",
    };
    let precision = match args.get("precision") {
        Some(val) => val.try_integer().map_err(|e| Error::chain("`precision` argument", e))?,
        None => 0,
    };
    let multiplier = if precision == 0 { 1.0 } else { 10.0_f64.powi(precision as i32) };

    match method {
        "common" => Ok(Value::Float((multiplier * num).round() / multiplier)),
        "ceil" => Ok(Value::Float((multiplier * num).ceil() / multiplier)),
        "floor" => Ok(Value::Float((multiplier * num).floor() / multiplier)),
        _ => Err(Error::msg(format!(
            "Filter `round` received an incorrect value for arg `method`: got `{:?}`, \
             only common, ceil and floor are allowed",
            method
        ))),
    }
}

/// Returns a human-readable file size (i.e. '110 MB') from an integer
pub fn filesizeformat(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let num = value.try_integer()?;
    num.file_size(file_size_opts::CONVENTIONAL)
        .or_else(|_| {
            Err(Error::msg(format!(
                "Filter `filesizeformat` was called on a negative number: {}",
                num
            )))
        })
        .map(Value::String)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::to_value;
    use std::collections::HashMap;

    #[test]
    fn test_pluralize_single() {
        let result = pluralize(&Value::Integer(1), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_pluralize_multiple() {
        let result = pluralize(&Value::Integer(2), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("s").unwrap());
    }

    #[test]
    fn test_pluralize_zero() {
        let result = pluralize(&Value::Integer(0), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("s").unwrap());
    }

    #[test]
    fn test_pluralize_multiple_custom_suffix() {
        let mut args = HashMap::new();
        args.insert("suffix".to_string(), to_value("es").unwrap());
        let result = pluralize(&Value::Integer(2), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("es").unwrap());
    }

    #[test]
    fn test_round_default() {
        let result = round(&Value::Float(2.1), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.0).unwrap());
    }

    #[test]
    fn test_round_default_precision() {
        let mut args = HashMap::new();
        args.insert("precision".to_string(), to_value(2).unwrap());
        let result = round(&Value::Float(3.15159265359), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3.15).unwrap());
    }

    #[test]
    fn test_round_ceil() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("ceil").unwrap());
        let result = round(&Value::Float(2.1), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(3.0).unwrap());
    }

    #[test]
    fn test_round_ceil_precision() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("ceil").unwrap());
        args.insert("precision".to_string(), to_value(1).unwrap());
        let result = round(&Value::Float(2.11), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.2).unwrap());
    }

    #[test]
    fn test_round_floor() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("floor").unwrap());
        let result = round(&Value::Float(2.1), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.0).unwrap());
    }

    #[test]
    fn test_round_floor_precision() {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value("floor").unwrap());
        args.insert("precision".to_string(), to_value(1).unwrap());
        let result = round(&Value::Float(2.91), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(2.9).unwrap());
    }

    #[test]
    fn test_filesizeformat() {
        let args = HashMap::new();
        let result = filesizeformat(Value::Integer(123456789), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("117.74 MB").unwrap());
    }
}
