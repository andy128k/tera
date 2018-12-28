/// Filters operating on array
use std::collections::HashMap;

use crate::context::{get_json_pointer, ValueRender};
use crate::errors::{Error, Result};
use crate::value::Value;
use crate::sort_utils::get_sort_strategy_for_type;

/// Returns the nth value of an array
/// If the array is empty, returns empty string
pub fn nth(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = value.try_array()?;

    if arr.is_empty() {
        return Ok(Value::String(String::new()));
    }

    let index = match args.get("n") {
        Some(val) => val.try_integer().map_err(|e| Error::chain("`n` argument", e))?,
        None => return Err(Error::msg("The `nth` filter has to have an `n` argument")),
    };
    if index < 0 {
        return Err(Error::msg(format!("`n` argument must be non-negative but {} was received", index)));
    }

    Ok(arr.get(index as usize).map_or_else(Value::empty_string, Clone::clone))
}

/// Returns the first value of an array
/// If the array is empty, returns empty string
pub fn first(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let arr = value.try_array()?;
    Ok(arr.first().map_or_else(Value::empty_string, Clone::clone))
}

/// Returns the last value of an array
/// If the array is empty, returns empty string
pub fn last(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let arr = value.try_array()?;
    Ok(arr.last().map_or_else(Value::empty_string, Clone::clone))
}

/// Joins all values in the array by the `sep` argument given
/// If no separator is given, it will use `""` (empty string) as separator
/// If the array is empty, returns empty string
pub fn join(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = value.try_array()?;
    let sep = match args.get("sep") {
        Some(val) => val.try_str().map_err(|e| Error::chain("sep argument", e))?,
        None => "",
    };

    // Convert all the values to strings before we join them together.
    let rendered = arr.iter().map(|val| val.render()).collect::<Vec<_>>();
    Ok(Value::String(rendered.join(sep)))
}

/// Sorts the array in ascending order.
/// Use the 'attribute' argument to define a field to sort by.
pub fn sort(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = value.try_array()?;
    if arr.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }

    let attribute = match args.get("attribute") {
        Some(val) => val.try_str().map_err(|e| Error::chain("`attribute` argument", e))?,
        None => "",
    };
    let ptr = match attribute {
        "" => "".to_string(),
        s => get_json_pointer(s),
    };

    let first = arr[0].pointer(&ptr).ok_or_else(|| {
        Error::msg(format!("attribute '{}' does not reference a field", attribute))
    })?;

    let mut strategy = get_sort_strategy_for_type(first)?;
    for v in arr {
        let key = v.pointer(&ptr).ok_or_else(|| {
            Error::msg(format!("attribute '{}' does not reference a field", attribute))
        })?;
        strategy.try_add_pair(v, key)?;
    }
    let sorted = strategy.sort();

    Ok(Value::Array(sorted))
}

/// Group the array values by the `attribute` given
/// Returns a hashmap of key => values, items without the `attribute` or where `attribute` is `null` are discarded.
/// The returned keys are stringified
pub fn group_by(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = value.try_array()?;
    if arr.is_empty() {
        return Ok(Value::Object(HashMap::new()));
    }

    let key = match args.get("attribute") {
        Some(val) => val.try_str().map_err(|e| Error::chain("`attribute` argument", e))?,
        None => return Err(Error::msg("The `group_by` filter has to have an `attribute` argument")),
    };

    let mut grouped = HashMap::new();

    for val in arr {
        if let Some(key_val) = val.pointer(key) {
            let str_key = format!("{}", key_val);
            grouped.entry(str_key).or_insert_with(Vec::new).push(val.clone());
        }
    }

    let obj = grouped
        .into_iter()
        .map(|(k, v)| (k, Value::Array(v)))
        .collect();

    Ok(Value::Object(obj))
}

/// Filter the array values, returning only the values where the `attribute` is equal to the `value`
/// Values without the `attribute` or with a null `attribute` are discarded
pub fn filter(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = value.try_array()?;
    if arr.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }

    let key = match args.get("attribute") {
        Some(val) => val.try_str().map_err(|e| Error::chain("`attribute` argument", e))?,
        None => return Err(Error::msg("The `filter` filter has to have an `attribute` argument")),
    };
    let value = match args.get("value") {
        Some(val) => val,
        None => return Err(Error::msg("The `filter` filter has to have a `value` argument")),
    };

    let arr = arr
        .iter()
        .filter(|v| v.pointer(key) == Some(value))
        .cloned()
        .collect();

    Ok(Value::Array(arr))
}

/// Slice the array
/// Use the `start` argument to define where to start (inclusive, default to `0`)
/// and `end` argument to define where to stop (exclusive, default to the length of the array)
/// `start` and `end` are 0-indexed
pub fn slice(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = value.try_array()?;
    if arr.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }

    let start = match args.get("start") {
        Some(val) => val.try_integer().map_err(|e| Error::chain("`start` argument", e))? as usize,
        None => 0,
    };
    // Not an error, but returns an empty Vec
    if start > arr.len() {
        return Ok(Value::Array(Vec::new()));
    }
    let mut end = match args.get("end") {
        Some(val) => val.try_integer().map_err(|e| Error::chain("`end` argument", e))? as usize,
        None => arr.len(),
    };
    if end > arr.len() {
        end = arr.len();
    }

    Ok(Value::Array(arr[start..end].to_vec()))
}

/// Concat the array with another one if the `with` parameter is an array or
/// just append it otherwise
pub fn concat(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
    let arr = value.try_array()?;

    let value = match args.get("with") {
        Some(val) => val,
        None => return Err(Error::msg("The `concat` filter has to have a `with` argument")),
    };

    let mut result = arr.to_vec();
    match value {
        Value::Array(vals) => {
            for val in vals {
                result.push(val.clone());
            }
        }
        _ => {
            result.push(value.clone());
        }
    }

    Ok(Value::Array(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::{to_value};
    use std::collections::HashMap;

    #[test]
    fn test_nth() {
        let mut args = HashMap::new();
        args.insert("n".to_string(), to_value(1).unwrap());
        let result = nth(&to_value(&vec![1, 2, 3, 4]).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&2).unwrap());
    }

    #[test]
    fn test_nth_empty() {
        let v: Vec<Value> = Vec::new();
        let mut args = HashMap::new();
        args.insert("n".to_string(), to_value(1).unwrap());
        let result = nth(&to_value(&v).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_first() {
        let result = first(&to_value(&vec![1, 2, 3, 4]).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&1).unwrap());
    }

    #[test]
    fn test_first_empty() {
        let v: Vec<Value> = Vec::new();

        let result = first(&to_value(&v).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_last() {
        let result = last(&to_value(&vec!["Hello", "World"]).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("World").unwrap());
    }

    #[test]
    fn test_last_empty() {
        let v: Vec<Value> = Vec::new();

        let result = last(&to_value(&v).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.ok().unwrap(), to_value("").unwrap());
    }

    #[test]
    fn test_join_sep() {
        let mut args = HashMap::new();
        args.insert("sep".to_owned(), to_value(&"==").unwrap());

        let result = join(&to_value(&vec!["Cats", "Dogs"]).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"Cats==Dogs").unwrap());
    }

    #[test]
    fn test_join_sep_omitted() {
        let result = join(&to_value(&vec![1.2, 3.4]).unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"1.23.4").unwrap());
    }

    #[test]
    fn test_join_empty() {
        let v: Vec<Value> = Vec::new();
        let mut args = HashMap::new();
        args.insert("sep".to_owned(), to_value(&"==").unwrap());

        let result = join(&to_value(&v).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&"").unwrap());
    }

    #[test]
    fn test_sort() {
        let v = to_value(vec![3, 1, 2, 5, 4]).unwrap();
        let args = HashMap::new();
        let result = sort(&v, &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(vec![1, 2, 3, 4, 5]).unwrap());
    }

    #[test]
    fn test_sort_empty() {
        let v = to_value(Vec::<f64>::new()).unwrap();
        let args = HashMap::new();
        let result = sort(&v, &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(Vec::<f64>::new()).unwrap());
    }

    #[derive(Serialize)]
    struct Foo {
        a: i32,
        b: i32,
    }

    #[test]
    fn test_sort_attribute() {
        let v = to_value(vec![
            Foo { a: 3, b: 5 },
            Foo { a: 2, b: 8 },
            Foo { a: 4, b: 7 },
            Foo { a: 1, b: 6 },
        ])
        .unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value(&"a").unwrap());

        let result = sort(&v, &args);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            to_value(vec![
                Foo { a: 1, b: 6 },
                Foo { a: 2, b: 8 },
                Foo { a: 3, b: 5 },
                Foo { a: 4, b: 7 },
            ])
            .unwrap()
        );
    }

    #[test]
    fn test_sort_invalid_attribute() {
        let v = to_value(vec![Foo { a: 3, b: 5 }]).unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value(&"invalid_field").unwrap());

        let result = sort(&v, &args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "attribute 'invalid_field' does not reference a field"
        );
    }

    #[test]
    fn test_sort_multiple_types() {
        let v = Value::Array(vec![Value::Integer(12), Value::Array(vec![])]);
        let args = HashMap::new();

        let result = sort(&v, &args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "expected number got []");
    }

    #[test]
    fn test_sort_non_finite_numbers() {
        let v = to_value(vec![
            ::std::f64::NEG_INFINITY, // NaN and friends get deserialized as Null by serde.
            ::std::f64::NAN,
        ])
        .unwrap();
        let args = HashMap::new();

        let result = sort(&v, &args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Null is not a sortable value");
    }

    #[derive(Serialize)]
    struct TupleStruct(i32, i32);

    #[test]
    fn test_sort_tuple() {
        let v = to_value(vec![
            TupleStruct(0, 1),
            TupleStruct(7, 0),
            TupleStruct(-1, 12),
            TupleStruct(18, 18),
        ])
        .unwrap();
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("0").unwrap());

        let result = sort(&v, &args);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            to_value(vec![
                TupleStruct(-1, 12),
                TupleStruct(0, 1),
                TupleStruct(7, 0),
                TupleStruct(18, 18),
            ])
            .unwrap()
        );
    }

    #[test]
    fn test_slice() {
        fn make_args(start: Option<usize>, end: Option<usize>) -> HashMap<String, Value> {
            let mut args = HashMap::new();
            if let Some(s) = start {
                args.insert("start".to_string(), to_value(s).unwrap());
            }
            if let Some(e) = end {
                args.insert("end".to_string(), to_value(e).unwrap());
            }
            args
        }

        let v = to_value(vec![1, 2, 3, 4, 5]).unwrap();

        let inputs = vec![
            (make_args(Some(1), None), vec![2, 3, 4, 5]),
            (make_args(None, Some(2)), vec![1, 2]),
            (make_args(Some(1), Some(2)), vec![2]),
            (make_args(None, None), vec![1, 2, 3, 4, 5]),
        ];

        for (args, expected) in inputs {
            let res = slice(&v, &args);
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), to_value(expected).unwrap());
        }
    }

    #[test]
    fn test_group_by() {
        let input = json!([
            {"id": 1, "year": 2015},
            {"id": 2, "year": 2015},
            {"id": 3, "year": 2016},
            {"id": 4, "year": 2017},
            {"id": 5, "year": 2017},
            {"id": 6, "year": 2017},
            {"id": 7, "year": 2018},
            {"id": 8},
            {"id": 9, "year": null},
        ]);
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("year").unwrap());

        let expected = json!({
            "2015": [{"id": 1, "year": 2015}, {"id": 2, "year": 2015}],
            "2016": [{"id": 3, "year": 2016}],
            "2017": [{"id": 4, "year": 2017}, {"id": 5, "year": 2017}, {"id": 6, "year": 2017}],
            "2018": [{"id": 7, "year": 2018}],
        });

        let res = group_by(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_group_by_nested_key() {
        let input = json!([
            {"id": 1, "company": {"id": 1}},
            {"id": 2, "company": {"id": 2}},
            {"id": 3, "company": {"id": 3}},
            {"id": 4, "company": {"id": 4}},
            {"id": 5, "company": {"id": 4}},
            {"id": 6, "company": {"id": 5}},
            {"id": 7, "company": {"id": 5}},
            {"id": 8},
            {"id": 9, "company": null},
        ]);
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("company.id").unwrap());

        let expected = json!({
            "1": [{"id": 1, "company": {"id": 1}}],
            "2": [{"id": 2, "company": {"id": 2}}],
            "3": [{"id": 3, "company": {"id": 3}}],
            "4": [{"id": 4, "company": {"id": 4}}, {"id": 5, "company": {"id": 4}}],
            "5": [{"id": 6, "company": {"id": 5}}, {"id": 7, "company": {"id": 5}}],
        });

        let res = group_by(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_filter_empty() {
        let res = filter(&json!([]), &HashMap::new());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), json!([]));
    }

    #[test]
    fn test_filter() {
        let input = json!([
            {"id": 1, "year": 2015},
            {"id": 2, "year": 2015},
            {"id": 3, "year": 2016},
            {"id": 4, "year": 2017},
            {"id": 5, "year": 2017},
            {"id": 6, "year": 2017},
            {"id": 7, "year": 2018},
            {"id": 8},
            {"id": 9, "year": null},
        ]);
        let mut args = HashMap::new();
        args.insert("attribute".to_string(), to_value("year").unwrap());
        args.insert("value".to_string(), to_value(2015).unwrap());

        let expected = json!([
            {"id": 1, "year": 2015},
            {"id": 2, "year": 2015},
        ]);

        let res = filter(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_concat_array() {
        let input = json!([1, 2, 3,]);
        let mut args = HashMap::new();
        args.insert("with".to_string(), json!([3, 4]));
        let expected = json!([1, 2, 3, 3, 4,]);

        let res = concat(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_concat_single_value() {
        let input = json!([1, 2, 3,]);
        let mut args = HashMap::new();
        args.insert("with".to_string(), json!(4));
        let expected = json!([1, 2, 3, 4,]);

        let res = concat(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }
}
