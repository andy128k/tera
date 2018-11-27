/// Filters operating on numbers
use std::collections::HashMap;

use humansize::{file_size_opts, FileSize};

use value::{Number, Value, ValueRef};
use errors::{Error, Result};

fn filter_value_error(filter_name: &str, value: &dyn Value, expected_type: &str) -> Error {
    Error::msg(format!(
        "Filter `{}` was called on an incorrect value: got `{:?}` but expected a {}",
        filter_name, value, expected_type
    ))
}

fn filter_arg_error(filter_name: &str, arg_name: &str, value: &dyn Value, expected_type: &str) -> Error {
    Error::msg(format!(
        "Filter `{}` received an incorrect type for arg `{}`: got `{:?}` but expected a {}",
        filter_name, arg_name, value, expected_type
    ))
}

/// Returns a suffix if the value is not equal to ±1. Suffix defaults to `s`
pub fn pluralize<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let num = value.as_number().ok_or_else(|| filter_value_error("pluralize", value, "Number"))?;

    let suffix = match args.get("suffix") {
        Some(val) => val.as_str().ok_or_else(|| filter_arg_error("pluralize", "suffix", &**val, "String"))?,
        None => "s",
    };

    // English uses plural when it isn't one
    let is_plural = match num {
        Number::Int(v) => v.abs() != 1,
        Number::UInt(v) => v != 1,
        Number::Float(v) => (v.abs() - 1.0).abs() > std::f64::EPSILON,
    };

    if is_plural {
        Ok(ValueRef::owned(suffix.to_owned()))
    } else {
        Ok(ValueRef::owned("".to_owned()))
    }
}

/// Returns a rounded number using the `method` arg and `precision` given.
/// `method` defaults to `common` which will round to the nearest number.
/// `ceil` and `floor` are also available as method.
/// `precision` defaults to `0`, meaning it will round to an integer
pub fn round<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let num = value.as_number().ok_or_else(|| filter_value_error("round", value, "Number"))?;

    if let Number::Float(num) = num {
        let method = match args.get("method") {
            Some(val) => val.as_str().ok_or_else(|| filter_arg_error("round", "method", &**val, "String"))?,
            None => "common",
        };
        let precision = match args.get("precision") {
            Some(val) => val.as_uint().ok_or_else(|| filter_arg_error("round", "precision", &**val, "uint"))?,
            None => 0,
        };
        let multiplier = if precision == 0 { 1.0 } else { 10.0_f64.powi(precision as i32) };

        match method.as_ref() {
            "common" => Ok(ValueRef::owned((multiplier * num).round() / multiplier)),
            "ceil" => Ok(ValueRef::owned((multiplier * num).ceil() / multiplier)),
            "floor" => Ok(ValueRef::owned((multiplier * num).floor() / multiplier)),
            _ => Err(Error::msg(format!(
                "Filter `round` received an incorrect value for arg `method`: got `{:?}`, \
                only common, ceil and floor are allowed",
                method
            ))),
        }
    } else {
        Ok(ValueRef::borrowed(value))
    }
}

/// Returns a human-readable file size (i.e. '110 MB') from an integer
pub fn filesizeformat<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let num = value.as_number().ok_or_else(|| filter_value_error("filesizeformat", value, "Number"))?;
    let result = match num {
        Number::Int(v) => v.file_size(file_size_opts::CONVENTIONAL),
        Number::UInt(v) => v.file_size(file_size_opts::CONVENTIONAL),
        Number::Float(v) => (v as u64).file_size(file_size_opts::CONVENTIONAL),
    }.or_else(|_| {
        Err(Error::msg(format!(
            "Filter `filesizeformat` was called on a negative number: {:?}",
            num
        )))
    })?;
    Ok(ValueRef::owned(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::to_value;
    use std::collections::HashMap;

    #[test]
    fn test_pluralize_single() {
        let result = pluralize(&to_value(1).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&""));
    }

    #[test]
    fn test_pluralize_multiple() {
        let result = pluralize(&to_value(2).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"s"));
    }

    #[test]
    fn test_pluralize_zero() {
        let result = pluralize(&to_value(0).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"s"));
    }

    #[test]
    fn test_pluralize_multiple_custom_suffix() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("suffix".to_string(), Box::new("es"));
        let result = pluralize(&to_value(2).unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"es"));
    }

    #[test]
    fn test_round_default() {
        let result = round(&to_value(2.1).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&2.0));
    }

    #[test]
    fn test_round_default_precision() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("precision".to_string(), Box::new(2));
        let result = round(&to_value(3.15159265359).unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&3.15));
    }

    #[test]
    fn test_round_ceil() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("method".to_string(), Box::new("ceil"));
        let result = round(&to_value(2.1).unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&3.0));
    }

    #[test]
    fn test_round_ceil_precision() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("method".to_string(), Box::new("ceil"));
        args.insert("precision".to_string(), Box::new(1));
        let result = round(&to_value(2.11).unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&2.2));
    }

    #[test]
    fn test_round_floor() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("method".to_string(), Box::new("floor"));
        let result = round(&to_value(2.1).unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&2.0));
    }

    #[test]
    fn test_round_floor_precision() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("method".to_string(), Box::new("floor"));
        args.insert("precision".to_string(), Box::new(1));
        let result = round(&to_value(2.91).unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&2.9));
    }

    #[test]
    fn test_filesizeformat() {
        let args = HashMap::new();
        let result = filesizeformat(&to_value(123456789).unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"117.74 MB"));
    }
}
