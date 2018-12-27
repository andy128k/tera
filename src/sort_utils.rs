use errors::{Error, Result};
use crate::value::Value;
use std::cmp::Ordering;

pub enum SortStrategy {
    Bools,
    Numbers,
    Strings,
    Arrays,
}

impl SortStrategy {
    pub fn cmp(&self, v1: &dyn Value, v2: &dyn Value) -> Result<Ordering> {
        match self {
            SortStrategy::Bools => {
                let b1 = v1.as_bool().ok_or_else(|| Error::msg(format!("expected bool got {:?}", v1)))?;
                let b2 = v2.as_bool().ok_or_else(|| Error::msg(format!("expected bool got {:?}", v2)))?;
                Ok(b1.cmp(&b2))
            },
            SortStrategy::Numbers => {
                let n1 = v1.as_number().ok_or_else(|| Error::msg(format!("expected number got {:?}", v1)))?;
                let n2 = v2.as_number().ok_or_else(|| Error::msg(format!("expected number got {:?}", v2)))?;
                if n1.eq(&n2) {
                    Ok(Ordering::Equal)
                } else if n1.lt(&n2) {
                    Ok(Ordering::Less)
                } else {
                    Ok(Ordering::Greater)
                }
            },
            SortStrategy::Strings => {
                let s1 = v1.as_str().ok_or_else(|| Error::msg(format!("expected string got {:?}", v1)))?;
                let s2 = v2.as_str().ok_or_else(|| Error::msg(format!("expected string got {:?}", v2)))?;
                Ok(s1.cmp(s2))
            },
            SortStrategy::Arrays => {
                let l1 = v1.len().ok_or_else(|| Error::msg(format!("expected array got {:?}", v1)))?;
                let l2 = v2.len().ok_or_else(|| Error::msg(format!("expected array got {:?}", v2)))?;
                Ok(l1.cmp(&l2))
            },
        }
    }
}

pub fn get_sort_strategy_for_type(ty: &dyn Value) -> Result<Box<SortStrategy>> {
    use Value::*;
    match *ty {
        Null => Err(Error::msg("Null is not a sortable value")),
        Bool(_) => Ok(Box::new(Bools::default())),
        Number(_) => Ok(Box::new(Numbers::default())),
        String(_) => Ok(Box::new(Strings::default())),
        Array(_) => Ok(Box::new(Arrays::default())),
        Object(_) => Err(Error::msg("Object is not a sortable value")),
    }
}
