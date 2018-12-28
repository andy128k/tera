use regex::Regex;
use crate::errors::{Error, Result};
use crate::value::Value;

/// The tester function type definition
pub trait Test: Sync + Send {
    /// The tester function type definition
    fn test(&self, value: Option<&Value>, args: &[Value]) -> Result<bool>;
}

impl<F> Test for F
where
    F: Fn(Option<&Value>, &[Value]) -> Result<bool> + Sync + Send,
{
    fn test(&self, value: Option<&Value>, args: &[Value]) -> Result<bool> {
        self(value, args)
    }
}

// Some helper functions to remove boilerplate with tester error handling
fn number_args_allowed(max: usize, args_len: usize) -> Result<()> {
    if max == 0 && args_len > max {
        return Err(Error::msg(format!(
            "Tester was called with some args but this test doesn't take args"
        )));
    }

    if args_len > max {
        return Err(Error::msg(format!(
            "Tester was called with {} args, the max number is {}",
            args_len, max
        )));
    }

    Ok(())
}

// Called to check if the Value is defined and return an Err if not
fn value_defined(tester_name: &str, value: Option<&Value>) -> Result<()> {
    if value.is_none() {
        return Err(Error::msg(format!(
            "Tester `{}` was called on an undefined variable",
            tester_name
        )));
    }

    Ok(())
}

/// Returns true if `value` is defined. Otherwise, returns false.
pub fn defined(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(0, params.len())?;

    Ok(value.is_some())
}

/// Returns true if `value` is undefined. Otherwise, returns false.
pub fn undefined(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(0, params.len())?;

    Ok(value.is_none())
}

/// Returns true if `value` is a string. Otherwise, returns false.
pub fn string(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(0, params.len())?;
    value_defined("string", value)?;

    match value {
        Some(Value::String(_)) => Ok(true),
        _ => Ok(false),
    }
}

/// Returns true if `value` is a number. Otherwise, returns false.
pub fn number(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(0, params.len())?;
    value_defined("number", value)?;

    match value {
        Some(Value::Integer(_)) |
        Some(Value::Float(_)) => Ok(true),
        _ => Ok(false),
    }
}

/// Returns true if `value` is an odd number. Otherwise, returns false.
pub fn odd(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(0, params.len())?;
    value_defined("odd", value)?;

    match value.and_then(|v| v.to_number().ok()) {
        Some(f) => Ok(f % 2.0 != 0.0),
        _ => Err(Error::msg("Tester `odd` was called on a variable that isn't a number")),
    }
}

/// Returns true if `value` is an even number. Otherwise, returns false.
pub fn even(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(0, params.len())?;
    value_defined("even", value)?;

    let is_odd = odd(value, params)?;
    Ok(!is_odd)
}

/// Returns true if `value` is divisible by the first param. Otherwise, returns false.
pub fn divisible_by(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(1, params.len())?;
    value_defined("divisibleby", value)?;

    match value.and_then(|v| v.to_number().ok()) {
        Some(val) => match params.first().and_then(|v| v.to_number().ok()) {
            Some(p) => Ok(val % p == 0.0),
            None => Err(Error::msg(
                "Tester `divisibleby` was called with a parameter that isn't a number",
            )),
        },
        None => {
            Err(Error::msg("Tester `divisibleby` was called on a variable that isn't a number"))
        }
    }
}

/// Returns true if `value` can be iterated over in Tera (ie is an array/tuple).
/// Otherwise, returns false.
pub fn iterable(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(0, params.len())?;
    value_defined("iterable", value)?;
    match value {
        Some(Value::Array(..)) => Ok(true),
        _ => Ok(false),
    }
}

// Helper function to extract string from an Option<Value> to remove boilerplate
// with tester error handling
fn extract_string<'a>(tester_name: &str, part: &str, value: Option<&'a Value>) -> Result<&'a str> {
    match value {
        Some(Value::String(ref s)) => Ok(s),
        None => Err(Error::msg(format!(
            "Tester `{}` was called {} that isn't a string",
            tester_name, part
        ))),
    }
}

/// Returns true if `value` starts with the given string. Otherwise, returns false.
pub fn starting_with(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(1, params.len())?;
    value_defined("starting_with", value)?;

    let value = extract_string("starting_with", "on a variable", value)?;
    let needle = extract_string("starting_with", "with a parameter", params.first())?;
    Ok(value.starts_with(needle))
}

/// Returns true if `value` ends with the given string. Otherwise, returns false.
pub fn ending_with(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(1, params.len())?;
    value_defined("ending_with", value)?;

    let value = extract_string("ending_with", "on a variable", value)?;
    let needle = extract_string("ending_with", "with a parameter", params.first())?;
    Ok(value.ends_with(needle))
}

/// Returns true if `value` contains the given argument. Otherwise, returns false.
pub fn containing(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(1, params.len())?;
    value_defined("containing", value)?;

    match value.unwrap() {
        Value::String(v) => {
            let needle = extract_string("containing", "with a parameter", params.first())?;
            Ok(v.contains(needle))
        }
        Value::Array(v) => Ok(v.contains(params.first().unwrap())),
        Value::Object(v) => {
            let needle = extract_string("containing", "with a parameter", params.first())?;
            Ok(v.contains_key(needle))
        }
        _ => Err(Error::msg("Tester `containing` can only be used on string, array or map")),
    }
}

/// Returns true if `value` is a string and matches the regex in the argument. Otherwise, returns false.
pub fn matching(value: Option<&Value>, params: &[Value]) -> Result<bool> {
    number_args_allowed(1, params.len())?;
    value_defined("matching", value)?;

    let value = extract_string("matching", "on a variable", value)?;
    let regex = extract_string("matching", "with a parameter", params.first())?;

    let regex = match Regex::new(regex) {
        Ok(regex) => regex,
        Err(err) => {
            return Err(Error::msg(format!(
                "Tester `matching`: Invalid regular expression: {}",
                err
            )));
        }
    };

    Ok(regex.is_match(value))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        containing, defined, divisible_by, ending_with, iterable, matching, starting_with, string,
    };

    use crate::value::Value;

    #[test]
    fn test_number_args_ok() {
        assert!(defined(None, &vec![]).is_ok())
    }

    #[test]
    fn test_too_many_args() {
        assert!(defined(None, &vec![Value::Integer(1)]).is_err())
    }

    #[test]
    fn test_value_defined() {
        assert!(string(None, &[]).is_err())
    }

    #[test]
    fn test_divisible_by() {
        let tests = vec![
            (1.0, 2.0, false),
            (4.0, 2.0, true),
            (4.0, 2.1, false),
            (10.0, 2.0, true),
            (10.0, 0.0, false),
        ];

        for (val, divisor, expected) in tests {
            assert_eq!(
                divisible_by(Some(&Value::Float(val)), &[Value::Float(divisor)],)
                    .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn test_iterable() {
        assert_eq!(iterable(Some(&Value::Array(vec![Value::String("1".to_string())])), &[]).unwrap(), true);
        assert_eq!(iterable(Some(&Value::Integer(1)), &[]).unwrap(), false);
        assert_eq!(iterable(Some(&Value::String("hello".to_string())), &[]).unwrap(), false);
    }

    #[test]
    fn test_starting_with() {
        assert!(starting_with(
            Some(&Value::String("helloworld".to_string())),
            &[Value::String("hello".to_string())],
        )
        .unwrap());
        assert!(
            !starting_with(Some(&Value::String("hello".to_string())), &[Value::String("hi".to_string())],).unwrap()
        );
    }

    #[test]
    fn test_ending_with() {
        assert!(
            ending_with(Some(&Value::String("helloworld".to_string())), &[Value::String("world".to_string())],)
                .unwrap()
        );
        assert!(
            !ending_with(Some(&Value::String("hello".to_string())), &[Value::String("hi".to_string())],).unwrap()
        );
    }

    #[test]
    fn test_containing() {
        let mut map = HashMap::new();
        map.insert("hey".to_string(), Value::Integer(1));

        let tests = vec![
            (Value::String("hello world".to_string()), Value::String("hel".to_string()), true),
            (Value::String("hello world".to_string()), Value::String("hol".to_string()), false),
            (Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]), Value::Integer(3), true),
            (Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]), Value::Integer(4), false),
            (Value::Object(map.clone()), Value::String("hey".to_string()), true),
            (Value::Object(map.clone()), Value::String("ho".to_string()), false),
        ];

        for (container, needle, expected) in tests {
            assert_eq!(containing(Some(&container), &[needle]).unwrap(), expected);
        }
    }

    #[test]
    fn test_matching() {
        let tests = vec![
            (Value::String("abc".to_string()), Value::String("b".to_string()), true),
            (Value::String("abc".to_string()), Value::String("^b$".to_string()), false),
            (
                Value::String("Hello, World!".to_string()),
                Value::String(r"(?i)(hello\W\sworld\W)".to_string()),
                true,
            ),
            (
                Value::String("The date was 2018-06-28".to_string()),
                Value::String(r"\d{4}-\d{2}-\d{2}$".to_string()),
                true,
            ),
        ];

        for (container, needle, expected) in tests {
            assert_eq!(matching(Some(&container), &[needle]).unwrap(), expected);
        }

        assert!(
            matching(Some(&Value::empty_string()), &[Value::String("(Invalid regex".to_string())]).is_err()
        );
    }
}
