use crate::errors::{Error, Result};
use crate::value::Value;
use std::cmp::Ordering;

#[derive(PartialEq, PartialOrd, Default, Copy, Clone)]
pub struct OrderedF64(f64);

impl OrderedF64 {
    fn new(n: f64) -> Result<Self> {
        if n.is_finite() {
            Ok(OrderedF64(n))
        } else {
            Err(Error::msg(format!("{} cannot be sorted", n)))
        }
    }
}

impl Eq for OrderedF64 {}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &OrderedF64) -> Ordering {
        // unwrap is safe because self.0 is finite.
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Default, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct ArrayLen(usize);

pub trait GetSortKey: Ord + Sized + Clone {
    fn get_sort_key(val: &Value) -> Result<Self>;
}

impl GetSortKey for i64 {
    fn get_sort_key(val: &Value) -> Result<Self> {
        val.try_integer()
    }
}

impl GetSortKey for OrderedF64 {
    fn get_sort_key(val: &Value) -> Result<Self> {
        let n = val.try_float()?;
        OrderedF64::new(n)
    }
}

impl GetSortKey for bool {
    fn get_sort_key(val: &Value) -> Result<Self> {
        val.try_bool()
    }
}

impl GetSortKey for String {
    fn get_sort_key(val: &Value) -> Result<Self> {
        let s = val.try_str()?;
        Ok(s.to_owned())
    }
}

impl GetSortKey for ArrayLen {
    fn get_sort_key(val: &Value) -> Result<Self> {
        let arr = val.try_array()?;
        Ok(ArrayLen(arr.len()))
    }
}

#[derive(Default)]
pub struct SortPairs<K: Ord> {
    pairs: Vec<(Value, K)>,
}

type Floats = SortPairs<OrderedF64>;
type Integers = SortPairs<i64>;
type Bools = SortPairs<bool>;
type Strings = SortPairs<String>;
type Arrays = SortPairs<ArrayLen>;

impl<K: GetSortKey> SortPairs<K> {
    fn try_add_pair(&mut self, val: &Value, key: &Value) -> Result<()> {
        let key = K::get_sort_key(key)?;
        self.pairs.push((val.clone(), key));
        Ok(())
    }

    fn sort(&mut self) -> Vec<Value> {
        self.pairs.sort_by_key(|a| a.1.clone());
        self.pairs.iter().map(|a| a.0.clone()).collect()
    }
}

pub trait SortStrategy {
    fn try_add_pair(&mut self, val: &Value, key: &Value) -> Result<()>;
    fn sort(&mut self) -> Vec<Value>;
}

impl<K: GetSortKey> SortStrategy for SortPairs<K> {
    fn try_add_pair(&mut self, val: &Value, key: &Value) -> Result<()> {
        SortPairs::try_add_pair(self, val, key)
    }

    fn sort(&mut self) -> Vec<Value> {
        SortPairs::sort(self)
    }
}

pub fn get_sort_strategy_for_type(ty: &Value) -> Result<Box<SortStrategy>> {
    use crate::Value::*;
    match *ty {
        Bool(_) => Ok(Box::new(Bools::default())),
        Integer(_) => Ok(Box::new(Integers::default())),
        Float(_) => Ok(Box::new(Floats::default())),
        String(_) => Ok(Box::new(Strings::default())),
        Array(_) => Ok(Box::new(Arrays::default())),
        Object(_) => Err(Error::msg("Object is not a sortable value")),
    }
}
