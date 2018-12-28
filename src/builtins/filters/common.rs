/// Filters operating on multiple types
use std::collections::HashMap;
use std::iter::FromIterator;

use crate::errors::{Error, Result};
use crate::value::Value;
use serde_json::{to_string, to_string_pretty};

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, Utc};

use crate::context::ValueRender;

// Returns the number of items in an array or the number of characters in a string.
// Returns 0 if not an array or string.
pub fn length(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Array(arr) => Ok(Value::Integer(arr.len() as i64)),
        Value::String(s) => Ok(Value::Integer(s.chars().count() as i64)),
        _ => Ok(Value::Integer(0)),
    }
}

// Reverses the elements of an array or the characters in a string.
pub fn reverse(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    match value {
        Value::Array(arr) => {
            let mut rev = arr.clone();
            rev.reverse();
            Ok(Value::Array(rev))
        }
        Value::String(s) => Ok(Value::String(String::from_iter(s.chars().rev()))),
        _ => Err(Error::msg(format!(
            "Filter `reverse` received an incorrect type for arg `value`: \
             got `{}` but expected Array|String",
            value
        ))),
    }
}

// Encodes a value of any type into json, optionally `pretty`-printing it
// `pretty` can be true to enable pretty-print, or omitted for compact printing
pub fn json_encode(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let pretty = args.get("pretty").and_then(|v| v.try_bool().ok()).unwrap_or(false);

    let json: serde_json::Value = value.clone().into();
    if pretty {
        to_string_pretty(&json).map(Value::String).map_err(Error::json)
    } else {
        to_string(&json).map(Value::String).map_err(Error::json)
    }
}

/// Returns a formatted time according to the given `format` argument.
/// `format` defaults to the ISO 8601 `YYYY-MM-DD` format.
///
/// Input can be an i64 timestamp (seconds since epoch) or an RFC3339 string
/// (default serialization format for `chrono::DateTime`).
///
/// a full reference for the time formatting syntax is available
/// on [chrono docs](https://lifthrasiir.github.io/rust-chrono/chrono/format/strftime/index.html)
pub fn date(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let format = match args.get("format") {
        Some(val) => val.try_str().map_err(|e| Error::chain("format argument", e))?,
        None => "%Y-%m-%d",
    };

    let formatted = match value {
        Value::Integer(i) => {
            NaiveDateTime::from_timestamp(*i, 0).format(format)
        },
        Value::String(s) => {
            if s.contains('T') {
                match s.parse::<DateTime<FixedOffset>>() {
                    Ok(val) => val.format(format),
                    Err(_) => match s.parse::<NaiveDateTime>() {
                        Ok(val) => val.format(format),
                        Err(_) => {
                            return Err(Error::msg(format!(
                                "Error parsing `{:?}` as rfc3339 date or naive datetime",
                                s
                            )));
                        }
                    },
                }
            } else {
                match NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                    Ok(val) => DateTime::<Utc>::from_utc(val.and_hms(0, 0, 0), Utc).format(format),
                    Err(_) => {
                        return Err(Error::msg(format!(
                            "Error parsing `{:?}` as YYYY-MM-DD date",
                            s
                        )));
                    }
                }
            }
        }
        _ => {
            return Err(Error::msg(format!(
                "Filter `date` received an incorrect type for arg `value`: \
                 got `{:?}` but expected i64|u64|String",
                value
            )));
        }
    };

    Ok(Value::String(formatted.to_string()))
}

// Returns the given value as a string.
pub fn as_str(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    Ok(Value::String(value.render().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Local};
    use serde_json;
    use serde_json::value::to_value;
    use std::collections::HashMap;

    #[test]
    fn as_str_object() {
        let map = Value::Object(HashMap::new());
        let result = as_str(&map, &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("[object]".to_owned()));
    }

    #[test]
    fn as_str_vec() {
        let arr = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
            Value::Integer(4),
        ]);
        let result = as_str(&arr, &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("[1, 2, 3, 4]".to_owned()));
    }

    #[test]
    fn length_vec() {
        let arr = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
            Value::Integer(4),
        ]);
        let result = length(&arr, &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Integer(4));
    }

    #[test]
    fn length_str() {
        let result = length(&Value::String("Hello World".to_string()), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Integer(11));
    }

    #[test]
    fn length_str_nonascii() {
        let result = length(&Value::String("日本語".to_string()), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Integer(3));
    }

    #[test]
    fn length_num() {
        let result = length(&Value::Integer(15), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Integer(0));
    }

    #[test]
    fn reverse_vec() {
        let result = reverse(&to_value(&vec![1, 2, 3, 4]).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&vec![4, 3, 2, 1]).unwrap());
    }

    #[test]
    fn reverse_str() {
        let result = reverse(&Value::String("Hello World".to_string()), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("dlroW olleH".to_string()));
    }

    #[test]
    fn reverse_num() {
        let result = reverse(&Value::Float(1.23), &HashMap::new());
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Filter `reverse` received an incorrect type for arg `value`: got `1.23` but expected Array|String"
        );
    }

    #[test]
    fn date_default() {
        let args = HashMap::new();
        let result = date(&Value::Integer(1482720453), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("2016-12-26".to_string()));
    }

    #[test]
    fn date_custom_format() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%Y-%m-%d %H:%M").unwrap());
        let result = date(&to_value(1482720453).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("2016-12-26 02:47").unwrap());
    }

    #[test]
    fn date_rfc3339() {
        let args = HashMap::new();
        let dt: DateTime<Local> = Local::now();
        let result = date(&Value::String(dt.to_rfc3339()), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String(dt.format("%Y-%m-%d").to_string()));
    }

    #[test]
    fn date_rfc3339_preserves_timezone() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), Value::String("%Y-%m-%d %z".to_string()));
        let result = date(&Value::String("1996-12-19T16:39:57-08:00".to_string()), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("1996-12-19 -0800".to_string()));
    }

    #[test]
    fn date_yyyy_mm_dd() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%a, %d %b %Y %H:%M:%S %z").unwrap());
        let result = date(&to_value("2017-03-05").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Sun, 05 Mar 2017 00:00:00 +0000").unwrap());
    }

    #[test]
    fn date_from_naive_datetime() {
        let mut args = HashMap::new();
        args.insert("format".to_string(), to_value("%a, %d %b %Y %H:%M:%S").unwrap());
        let result = date(&to_value("2017-03-05T00:00:00.602").unwrap(), &args);
        println!("{:?}", result);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Sun, 05 Mar 2017 00:00:00").unwrap());
    }

    #[test]
    fn test_json_encode() {
        let args = HashMap::new();
        let result =
            json_encode(&serde_json::from_str("{\"key\": [\"value1\", 2, true]}").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("{\"key\":[\"value1\",2,true]}").unwrap());
    }

    #[test]
    fn test_json_encode_pretty() {
        let mut args = HashMap::new();
        args.insert("pretty".to_string(), to_value(true).unwrap());
        let result =
            json_encode(&serde_json::from_str("{\"key\": [\"value1\", 2, true]}").unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            to_value("{\n  \"key\": [\n    \"value1\",\n    2,\n    true\n  ]\n}").unwrap()
        );
    }
}
