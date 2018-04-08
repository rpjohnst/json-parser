use std::{fmt, result};
use lex::{Lex, Token, TokenKind};
use json;

/// A JSON parser.
///
/// value = STRING | NUMBER | BOOL | NULL | object | array
///
/// object = '{' '}'| '{' pairs '}'
/// pairs = pair | pairs ',' pair
/// pair = STRING ':' value
///
/// array = '[' ']' | '[' elements ']'
/// elements = value | elements ',' value
pub struct Parse<'source> {
    lex: Lex<'source>,
}

pub type Result<'source, T> = result::Result<T, ParseError<'source>>;

/// An unexpected token.
pub struct ParseError<'source> {
    token: Token<'source>,
}

impl<'source> fmt::Debug for ParseError<'source> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unexpected token {:?}", self.token)?;
        Ok(())
    }
}

struct Value(json::Value);
struct Object(json::Object);
struct Pairs(json::Object);
struct Pair((String, json::Value));
struct Array(json::Array);
struct Elements(json::Array);

enum Either<T, U> {
    Left(T),
    Right(U),
}

impl<'source> Parse<'source> {
    /// Create a new JSON parser for the given string.
    ///
    /// The entire string should consist of a single JSON value.
    pub fn new(source: &'source str) -> Self {
        let lex = Lex::new(source);
        Parse { lex }
    }

    /// Parse a JSON value.
    pub fn value(&mut self) -> Result<'source, json::Value> {
        let Value(value) = self.goal_start()?;
        Ok(value)
    }

    /// S0 = value = * STRING
    ///      value = * NUMBER
    ///      value = * BOOL
    ///      value = * NULL
    ///      value = * object
    ///      value = * array
    ///      object = * '{' pairs '}'
    ///      object = * '{' '}'
    ///      array = * '[' elements ']'
    ///      array = * '[' ']'
    fn goal_start(&mut self) -> Result<'source, Value> {
        let token = self.lex.token();
        let value = match token {
            Token { kind: TokenKind::String(string), .. } => self.value_string(string)?,
            Token { kind: TokenKind::Number(number), .. } => self.value_number(number)?,
            Token { kind: TokenKind::Bool(bool_), .. } => self.value_bool(bool_)?,
            Token { kind: TokenKind::Null, .. } => self.value_null()?,
            Token { kind: TokenKind::LeftBrace, .. } => {
                let object = self.object_open()?;
                self.value_object(object)?
            }
            Token { kind: TokenKind::LeftBracket, .. } => {
                let array = self.array_open()?;
                self.value_array(array)?
            }
            _ => return Err(ParseError { token }),
        };
        Ok(self.goal_value(value)?)
    }

    fn goal_value(&mut self, value: Value) -> Result<'source, Value> {
        let token = self.lex.token();
        match token {
            Token { kind: TokenKind::End, .. } => Ok(value),
            _ => return Err(ParseError { token }),
        }
    }

    /// S1 = value = STRING *
    fn value_string(&mut self, string: String) -> Result<'source, Value> {
        let value = json::Value::String(string);
        Ok(Value(value))
    }

    /// S2 = value = NUMBER *
    fn value_number(&mut self, number: f64) -> Result<'source, Value> {
        let value = json::Value::Number(number);
        Ok(Value(value))
    }

    /// S3 = value = BOOL *
    fn value_bool(&mut self, bool_: bool) -> Result<'source, Value> {
        let value = json::Value::Bool(bool_);
        Ok(Value(value))
    }

    /// S4 = value = NULL *
    fn value_null(&mut self) -> Result<'source, Value> {
        let value = json::Value::Null;
        Ok(Value(value))
    }

    /// S5 = object = '{' * pairs '}'
    ///      object = '{' * '}'
    ///      pairs = * pair
    ///      pairs = * pairs ',' pair
    ///      pair = * STRING ':' value
    fn object_open(&mut self) -> Result<'source, Object> {
        let token = self.lex.token();
        let mut pairs = match token {
            Token { kind: TokenKind::String(string), .. } => {
                let pair = self.pair_string(string)?;
                self.pairs_pair(pair)?
            }
            Token { kind: TokenKind::RightBrace, .. } => return Ok(self.object_open_close()?),
            _ => return Err(ParseError { token }),
        };
        loop {
            match self.object_open_pairs(pairs)? {
                Either::Left(p) => pairs = p,
                Either::Right(object) => return Ok(object),
            }
        }
    }

    /// S6 = pair = STRING * ':' value
    fn pair_string(&mut self, string: String) -> Result<'source, Pair> {
        let token = self.lex.token();
        match token {
            Token { kind: TokenKind::Colon, .. } => Ok(self.pair_string_colon(string)?),
            _ => return Err(ParseError { token }),
        }
    }

    /// S7 = pair = STRING ':' * value
    ///      value = * STRING
    ///      value = * NUMBER
    ///      value = * BOOL
    ///      value = * NULL
    ///      value = * object
    ///      value = * array
    ///      object = * '{' pairs '}'
    ///      object = * '{' '}'
    ///      array = * '[' elements ']'
    ///      array = * '[' ']'
    fn pair_string_colon(&mut self, string: String) -> Result<'source, Pair> {
        let token = self.lex.token();
        let value = match token {
            Token { kind: TokenKind::String(string), .. } => self.value_string(string)?,
            Token { kind: TokenKind::Number(number), .. } => self.value_number(number)?,
            Token { kind: TokenKind::Bool(bool_), .. } => self.value_bool(bool_)?,
            Token { kind: TokenKind::Null, .. } => self.value_null()?,
            Token { kind: TokenKind::LeftBrace, .. } => {
                let object = self.object_open()?;
                self.value_object(object)?
            }
            Token { kind: TokenKind::LeftBracket, .. } => {
                let array = self.array_open()?;
                self.value_array(array)?
            }
            _ => return Err(ParseError { token }),
        };
        Ok(self.pair_string_colon_value(string, value)?)
    }

    /// S8 = pair = STRING ':' value *
    fn pair_string_colon_value(&mut self, string: String, value: Value) -> Result<'source, Pair> {
        let Value(value) = value;
        let pair = (string, value);
        Ok(Pair(pair))
    }

    /// S9 = pairs = pair *
    fn pairs_pair(&mut self, pair: Pair) -> Result<'source, Pairs> {
        let Pair((key, value)) = pair;
        let mut object = json::Object::new();
        object.insert(key, value);
        Ok(Pairs(object))
    }

    /// S10= object = '{' pairs * '}'
    ///      pairs = pairs * ',' pair
    fn object_open_pairs(&mut self, pairs: Pairs) -> Result<'source, Either<Pairs, Object>> {
        let token = self.lex.token();
        match token {
            Token { kind: TokenKind::Comma, .. } => {
                let pairs = self.pairs_pairs_comma(pairs)?;
                Ok(Either::Left(pairs))
            }
            Token { kind: TokenKind::RightBrace, .. } => {
                let object = self.object_open_pairs_close(pairs)?;
                Ok(Either::Right(object))
            }
            _ => return Err(ParseError { token }),
        }
    }

    /// S11= pairs = pairs ',' * pair
    ///      pair = * STRING ':' value
    fn pairs_pairs_comma(&mut self, pairs: Pairs) -> Result<'source, Pairs> {
        let token = self.lex.token();
        let pair = match token {
            Token { kind: TokenKind::String(string), .. } => self.pair_string(string)?,
            _ => return Err(ParseError { token }),
        };
        Ok(self.pairs_pairs_comma_pair(pairs, pair)?)
    }

    /// S12= pairs = pairs ',' pair *
    fn pairs_pairs_comma_pair(&mut self, pairs: Pairs, pair: Pair) -> Result<'source, Pairs> {
        let Pairs(mut object) = pairs;
        let Pair((key, value)) = pair;
        object.insert(key, value);
        Ok(Pairs(object))
    }

    /// S13= object = '{' pairs '}' *
    fn object_open_pairs_close(&mut self, pairs: Pairs) -> Result<'source, Object> {
        let Pairs(object) = pairs;
        Ok(Object(object))
    }

    /// S14= object = '{' '}' *
    fn object_open_close(&mut self) -> Result<'source, Object> {
        let object = json::Object::new();
        Ok(Object(object))
    }

    /// S15= value = object *
    fn value_object(&mut self, object: Object) -> Result<'source, Value> {
        let Object(object) = object;
        let value = json::Value::Object(object);
        Ok(Value(value))
    }

    /// S16= array = '[' * elements ']'
    ///      array = '[' * ']'
    ///      elements = * value
    ///      elements = * elements ',' value
    ///      value = * STRING
    ///      value = * NUMBER
    ///      value = * BOOL
    ///      value = * NULL
    ///      value = * object
    ///      value = * array
    ///      object = * '{' pairs '}'
    ///      object = * '{' '}'
    ///      array = * '[' elements ']'
    ///      array = * '[' ']'
    fn array_open(&mut self) -> Result<'source, Array> {
        let token = self.lex.token();
        let value = match token {
            Token { kind: TokenKind::String(string), .. } => self.value_string(string)?,
            Token { kind: TokenKind::Number(number), .. } => self.value_number(number)?,
            Token { kind: TokenKind::Bool(bool_), .. } => self.value_bool(bool_)?,
            Token { kind: TokenKind::Null, .. } => self.value_null()?,
            Token { kind: TokenKind::LeftBrace, .. } => {
                let object = self.object_open()?;
                self.value_object(object)?
            }
            Token { kind: TokenKind::LeftBracket, .. } => {
                let array = self.array_open()?;
                self.value_array(array)?
            }
            Token { kind: TokenKind::RightBracket, .. } => return Ok(self.array_open_close()?),
            _ => return Err(ParseError { token }),
        };
        let mut elements = self.elements_value(value)?;
        loop {
            match self.array_open_elements(elements)? {
                Either::Left(e) => elements = e,
                Either::Right(array) => return Ok(array),
            }
        }
    }

    /// S17= elements = value *
    fn elements_value(&mut self, value: Value) -> Result<'source, Elements> {
        let Value(value) = value;
        let mut array = json::Array::new();
        array.push(value);
        Ok(Elements(array))
    }

    /// S18= array = '[' elements * ']'
    ///      elements = elements * ',' value
    fn array_open_elements(&mut self, elements: Elements) -> Result<'source, Either<Elements, Array>> {
        let token = self.lex.token();
        match token {
            Token { kind: TokenKind::Comma, .. } => {
                let elements = self.elements_elements_comma(elements)?;
                Ok(Either::Left(elements))
            }
            Token { kind: TokenKind::RightBracket, .. } => {
                let array = self.array_open_elements_close(elements)?;
                Ok(Either::Right(array))
            }
            _ => return Err(ParseError { token }),
        }
    }

    /// S19= elements = elements ',' * value
    ///      value = * STRING
    ///      value = * NUMBER
    ///      value = * BOOL
    ///      value = * NULL
    ///      value = * object
    ///      value = * array
    ///      object = * '{' pairs '}'
    ///      object = * '{' '}'
    ///      array = * '[' elements ']'
    ///      array = * '[' ']'
    fn elements_elements_comma(&mut self, elements: Elements) -> Result<'source, Elements> {
        let token = self.lex.token();
        let value = match token {
            Token { kind: TokenKind::String(string), .. } => self.value_string(string)?,
            Token { kind: TokenKind::Number(number), .. } => self.value_number(number)?,
            Token { kind: TokenKind::Bool(bool_), .. } => self.value_bool(bool_)?,
            Token { kind: TokenKind::Null, .. } => self.value_null()?,
            Token { kind: TokenKind::LeftBrace, .. } => {
                let object = self.object_open()?;
                self.value_object(object)?
            }
            Token { kind: TokenKind::LeftBracket, .. } => {
                let array = self.array_open()?;
                self.value_array(array)?
            }
            _ => return Err(ParseError { token }),
        };
        Ok(self.elements_elements_comma_value(elements, value)?)
    }

    /// S20= elements = elements ',' value *
    fn elements_elements_comma_value(&mut self, elements: Elements, value: Value) -> Result<'source, Elements> {
        let Elements(mut array) = elements;
        let Value(value) = value;
        array.push(value);
        Ok(Elements(array))
    }

    /// S21= array = '[' elements ']' *
    fn array_open_elements_close(&mut self, elements: Elements) -> Result<'source, Array> {
        let Elements(array) = elements;
        Ok(Array(array))
    }

    /// S22= array = '[' ']' *
    fn array_open_close(&mut self) -> Result<'source, Array> {
        let array = json::Array::new();
        Ok(Array(array))
    }

    /// S23 = value = array *
    fn value_array(&mut self, array: Array) -> Result<'source, Value> {
        let Array(array) = array;
        let value = json::Value::Array(array);
        Ok(Value(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let s = r#"{ "foo": 3, "bar": ["baz", -5.8], "qux": 13e5 }"#;
        assert!(Parse::new(s).value().is_ok());
    }
}
