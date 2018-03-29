use std::collections::HashMap;

/// A JSON value.
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    Object(Object),
    Array(Array),
}

pub type Object = HashMap<String, Value>;

pub type Array = Vec<Value>;
