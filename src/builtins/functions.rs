use std::collections::HashMap;

use chrono::prelude::*;
use value::Value;

use errors::{Error, Result};

/// The global function type definition
pub trait Function: Sync + Send {
    /// The global function type definition
    fn call(&self, args: &HashMap<String, Box<dyn Value>>) -> Result<Box<dyn Value>>;
}

impl<F> Function for F
where
    F: Fn(&HashMap<String, Box<dyn Value>>) -> Result<Box<dyn Value>> + Sync + Send,
{
    fn call(&self, args: &HashMap<String, Box<dyn Value>>) -> Result<Box<dyn Value>> {
        self(args)
    }
}

pub fn range(args: &HashMap<String, Box<dyn Value>>) -> Result<Box<dyn Value>> {
    let start = match args.get("start") {
        Some(val) => match val.as_uint() {
            Some(v) => v,
            None => {
                return Err(Error::msg(format!(
                    "Global function `range` received start={:?} but `start` can only be a number",
                    val
                )));
            }
        },
        None => 0,
    };
    let step_by = match args.get("step_by") {
        Some(val) => match val.as_uint() {
            Some(v) => v,
            None => {
                return Err(Error::msg(format!(
                    "Global function `range` received step_by={:?} but `step` can only be a number",
                    val
                )));
            }
        },
        None => 1,
    };
    let end = match args.get("end") {
        Some(val) => match val.as_uint() {
            Some(v) => v,
            None => {
                return Err(Error::msg(format!(
                    "Global function `range` received end={:?} but `end` can only be a number",
                    val
                )));
            }
        },
        None => {
            return Err(Error::msg("Global function `range` was called without a `end` argument"));
        }
    };

    if start > end {
        return Err(Error::msg("Global function `range` was called without a `start` argument greater than the `end` one"));
    }

    let mut i = start;
    let mut res = vec![];
    while i < end {
        res.push(i);
        i += step_by;
    }
    Ok(Box::new(res))
}

pub fn now(args: &HashMap<String, Box<dyn Value>>) -> Result<Box<dyn Value>> {
    let utc = match args.get("utc") {
        Some(val) => match val.as_bool() {
            Some(v) => v,
            None => {
                return Err(Error::msg(format!(
                    "Global function `now` received utc={:?} but `utc` can only be a boolean",
                    val
                )));
            }
        },
        None => false,
    };
    let timestamp = match args.get("timestamp") {
        Some(val) => match val.as_bool() {
            Some(v) => v,
            None => {
                return Err(Error::msg(format!(
                    "Global function `now` received timestamp={:?} but `timestamp` can only be a boolean",
                    val
                )));
            }
        },
        None => false,
    };

    if utc {
        let datetime = Utc::now();
        if timestamp {
            return Ok(Box::new(datetime.timestamp()));
        }
        Ok(Box::new(datetime.to_rfc3339()))
    } else {
        let datetime = Local::now();
        if timestamp {
            return Ok(Box::new(datetime.timestamp()));
        }
        Ok(Box::new(datetime.to_rfc3339()))
    }
}

pub fn throw(args: &HashMap<String, Box<dyn Value>>) -> Result<Box<dyn Value>> {
    match args.get("message") {
        Some(val) => match val.as_str() {
            Some(v) => Err(Error::msg(v)),
            None => Err(Error::msg(format!(
                "Global function `throw` received message={:?} but `message` can only be a string",
                val
            ))),
        },
        None => Err(Error::msg("Global function `throw` was called without a `message` argument")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;

    #[test]
    fn range_default() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("end".to_string(), Box::new(5));

        let res = range(&args).unwrap();
        assert!(res.eq(&vec![0, 1, 2, 3, 4]));
    }

    #[test]
    fn range_start() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("end".to_string(), Box::new(5));
        args.insert("start".to_string(), Box::new(1));

        let res = range(&args).unwrap();
        assert!(res.eq(&vec![1, 2, 3, 4]));
    }

    #[test]
    fn range_start_greater_than_end() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("end".to_string(), Box::new(5));
        args.insert("start".to_string(), Box::new(6));

        assert!(range(&args).is_err());
    }

    #[test]
    fn range_step_by() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("end".to_string(), Box::new(10));
        args.insert("step_by".to_string(), Box::new(2));

        let res = range(&args).unwrap();
        assert!(res.eq(&vec![0, 2, 4, 6, 8]));
    }

    #[test]
    fn now_default() {
        let args = HashMap::new();

        let res = now(&args).unwrap();
        assert!(res.as_str().unwrap().contains("T"));
    }

    #[test]
    fn now_datetime_utc() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("utc".to_string(), Box::new(true));

        let res = now(&args).unwrap();
        assert!(res.as_str().is_some());
        let val = res.as_str().unwrap();
        assert!(val.contains("T"));
        assert!(val.contains("+00:00"));
    }

    #[test]
    fn now_timestamp() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("timestamp".to_string(), Box::new(true));

        let res = now(&args).unwrap();
        assert!(res.to_number().is_some());
    }

    #[test]
    fn throw_errors_with_message() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("message".to_string(), Box::new("Hello".to_string()));

        let res = throw(&args);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.to_string(), "Hello");
    }
}
