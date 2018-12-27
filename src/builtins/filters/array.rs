/// Filters operating on array
use std::collections::HashMap;

use context::ValueRender;
use errors::{Error, Result};
use serde_json::value::to_value;
use value::{Value, ValueRef};
use sort_utils::get_sort_strategy_for_type;

fn ensure_value_is_array(filter_name: &str, value: &dyn Value) -> Result<()> {
    if value.is_array() {
        Ok(())
    } else {
        Err(Error::msg(format!(
            "Filter `{}` was called on an incorrect value: got `{:?}` but expected an array",
            filter_name, value
        )))
    }
}

fn filter_arg_error(filter_name: &str, arg_name: &str, value: &dyn Value, expected_type: &str) -> Error {
    Error::msg(format!(
        "Filter `{}` received an incorrect type for arg `{}`: got `{:?}` but expected a {}",
        filter_name, arg_name, value, expected_type
    ))
}

/// Returns the nth value of an array
/// If the array is empty, returns empty string
pub fn nth<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    ensure_value_is_array("nth", value)?;

    if value.len() == Some(0) {
        return Ok(ValueRef::borrowed(&""));
    }

    let index = match args.get("n") {
        Some(val) => val.as_uint().ok_or_else(|| filter_arg_error("nth", "n", &**val, "uint"))? as usize,
        None => return Err(Error::msg("The `nth` filter has to have an `n` argument")),
    };

    Ok(ValueRef::borrowed(value.get(index).unwrap_or(&"")))
}

/// Returns the first value of an array
/// If the array is empty, returns empty string
pub fn first<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    ensure_value_is_array("first", value)?;
    Ok(ValueRef::borrowed(value.get(0).unwrap_or(&"")))
}

/// Returns the last value of an array
/// If the array is empty, returns empty string
pub fn last<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    ensure_value_is_array("last", value)?;
    let last_index = value.len().map_or(0, |l| l - 1);
    Ok(ValueRef::borrowed(value.get(last_index).unwrap_or(&"")))
}

/// Joins all values in the array by the `sep` argument given
/// If no separator is given, it will use `""` (empty string) as separator
/// If the array is empty, returns empty string
pub fn join<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    ensure_value_is_array("join", value)?;
    let sep = match args.get("sep") {
        Some(val) => val.as_str().ok_or_else(|| filter_arg_error("truncate", "sep", &**val, "String"))?,
        None => "",
    };

    // Convert all the values to strings before we join them together.
    let rendered = (0..value.len().unwrap()).into_iter().map(|index| value.get(index).unwrap().render()).collect::<Vec<_>>();
    Ok(ValueRef::owned(rendered.join(sep)))
}

/// Sorts the array in ascending order.
/// Use the 'attribute' argument to define a field to sort by.
pub fn sort<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    ensure_value_is_array("sort", value)?;
    if value.len() == Some(0) {
        return Ok(ValueRef::borrowed(value));
    }

    let attribute = match args.get("attribute") {
        Some(val) => val.as_str().ok_or_else(|| filter_arg_error("sort", "attribute", &**val, "String"))?,
        None => "",
    };

    let first = value.get(0).unwrap().get_by_pointer(attribute).ok_or_else(|| {
        Error::msg(format!("attribute '{}' does not reference a field", attribute))
    })?;

    let mut result: Vec<&'v dyn Value> = Vec::new();
    for index in 0..value.len().unwrap() {
        result.push(value.get(index).unwrap());
    }

    let strategy = get_sort_strategy_for_type(first)?;
    result.sort_by(|v1, v2| {
        let key1 = v1.get_by_pointer(attribute).ok_or_else(|| {
            Error::msg(format!("attribute '{}' does not reference a field", attribute))
        }).unwrap();
        let key2 = v2.get_by_pointer(attribute).ok_or_else(|| {
            Error::msg(format!("attribute '{}' does not reference a field", attribute))
        }).unwrap();
        strategy.cmp(key1, key2).unwrap()
    });

    Ok(ValueRef::owned(result))
}

/// Group the array values by the `attribute` given
/// Returns a hashmap of key => values, items without the `attribute` or where `attribute` is `null` are discarded.
/// The returned keys are stringified
pub fn group_by<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    ensure_value_is_array("group_by", value)?;
    if value.len() == Some(0) {
        return Ok(ValueRef::owned(HashMap::new()));
    }

    let key = match args.get("attribute") {
        Some(val) => val.as_str().ok_or_else(|| filter_arg_error("group_by", "attribute", &**val, "String"))?,
        None => return Err(Error::msg("The `group_by` filter has to have an `attribute` argument")),
    };

    let mut grouped = HashMap::new();

    for index in 0..value.len().unwrap() {
        let val = value.get(index).unwrap();
        if let Some(key_val) = val.get_by_pointer(key) {
            if key_val.is_null() {
                continue;
            }
            let str_key = format!("{}", key_val);

            if let Some(vals) = grouped.get_mut(&str_key) {
                vals.as_array_mut().unwrap().push(val);
                continue;
            }
            grouped.insert(str_key, vec![val]);
        }
    }

    Ok(ValueRef::owned(grouped))
}

/// Filter the array values, returning only the values where the `attribute` is equal to the `value`
/// Values without the `attribute` or with a null `attribute` are discarded
pub fn filter<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    ensure_value_is_array("filter", value)?;
    if value.len() == Some(0) {
        return Ok(ValueRef::borrowed(value));
    }

    let key = match args.get("attribute") {
        Some(val) => val.as_str().ok_or_else(|| filter_arg_error("filter", "attribute", &**val, "String"))?,
        None => return Err(Error::msg("The `filter` filter has to have an `attribute` argument")),
    };
    let value = match args.get("value") {
        Some(val) => val,
        None => return Err(Error::msg("The `filter` filter has to have a `value` argument")),
    };

    arr = arr
        .into_iter()
        .filter(|v| {
            if let Some(val) = v.get_by_pointer(key) {
                if val.is_null() {
                    false
                } else {
                    val == value
                }
            } else {
                false
            }
        })
        .collect::<Vec<_>>();

    Ok(to_value(arr).unwrap())
}

/// Slice the array
/// Use the `start` argument to define where to start (inclusive, default to `0`)
/// and `end` argument to define where to stop (exclusive, default to the length of the array)
/// `start` and `end` are 0-indexed
pub fn slice<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    ensure_value_is_array("slice", value)?;
    if value.len() == Some(0) {
        return Ok(ValueRef::borrowed(value));
    }

    let start = match args.get("start") {
        Some(val) => try_get_value!("slice", "start", f64, val) as usize,
        None => 0,
    };
    // Not an error, but returns an empty Vec
    if start > arr.len() {
        return Ok(Vec::<Value>::new().into());
    }
    let mut end = match args.get("end") {
        Some(val) => try_get_value!("slice", "end", f64, val) as usize,
        None => arr.len(),
    };
    if end > arr.len() {
        end = arr.len();
    }

    Ok(arr[start..end].into())
}

/// Concat the array with another one if the `with` parameter is an array or
/// just append it otherwise
pub fn concat<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let mut arr = try_get_value!("concat", "value", Vec<Value>, value);

    let value = match args.get("with") {
        Some(val) => val,
        None => return Err(Error::msg("The `concat` filter has to have a `with` argument")),
    };

    if value.is_array() {
        match value {
            Value::Array(vals) => {
                for val in vals {
                    arr.push(val.clone());
                }
            }
            _ => unreachable!("Got something other than an array??"),
        }
    } else {
        arr.push(value.clone());
    }

    Ok(to_value(arr).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::value::{to_value};
    use std::collections::HashMap;

    #[test]
    fn test_nth() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("n".to_string(), to_value(1).unwrap());
        let result = nth(&to_value(&vec![1, 2, 3, 4]).unwrap(), &args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&2).unwrap());
    }

    #[test]
    fn test_nth_empty() {
        let v: Vec<Value> = Vec::new();
        let mut args = HashMap::<String, Box<dyn Value>>::new();
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
        let mut args = HashMap::<String, Box<dyn Value>>::new();
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
        let mut args = HashMap::<String, Box<dyn Value>>::new();
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
        let mut args = HashMap::<String, Box<dyn Value>>::new();
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
        let mut args = HashMap::<String, Box<dyn Value>>::new();
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
        let v = to_value(vec![Value::Number(12.into()), Value::Array(vec![])]).unwrap();
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
        let mut args = HashMap::<String, Box<dyn Value>>::new();
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
        fn make_args(start: Option<usize>, end: Option<usize>) -> HashMap<String, Box<dyn Value>> {
            let mut args = HashMap::<String, Box<dyn Value>>::new();
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
        let mut args = HashMap::<String, Box<dyn Value>>::new();
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
        let mut args = HashMap::<String, Box<dyn Value>>::new();
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
        let mut args = HashMap::<String, Box<dyn Value>>::new();
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
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("with".to_string(), json!([3, 4]));
        let expected = json!([1, 2, 3, 3, 4,]);

        let res = concat(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }

    #[test]
    fn test_concat_single_value() {
        let input = json!([1, 2, 3,]);
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("with".to_string(), json!(4));
        let expected = json!([1, 2, 3, 4,]);

        let res = concat(&input, &args);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), to_value(expected).unwrap());
    }
}
