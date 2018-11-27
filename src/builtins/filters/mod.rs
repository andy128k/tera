use std::collections::HashMap;

use errors::Result;
use value::{Value, ValueRef};

pub mod array;
pub mod common;
pub mod number;
pub mod object;
pub mod string;

/// The filter function type definition
pub trait Filter: Sync + Send {
    /// The filter function type definition
    fn filter<'v>(&self, value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>>;
}

impl<F> Filter for F
where
    for<'f> F: Fn(&'f dyn Value, &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'f>> + Sync + Send,
{
    fn filter<'v>(&self, value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
        self(value, args)
    }
}
