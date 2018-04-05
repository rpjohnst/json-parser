use lex::{Lex, Token, TokenKind};
use json;

/// A JSON parser.
///
/// value = STRING | NUMBER | BOOL | NULL | object | array
///
/// object = '{' pairs '}' | '{' '}'
/// pairs = pair | pairs ',' pair
/// pair = STRING ':' value
///
/// array = '[' elements ']' | '[' ']'
/// elements = value | elements ',' value
pub struct Parse<'source> {
    lex: Lex<'source>,
}

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
    pub fn value(&mut self) -> Result<json::Value, ()> {
        Ok(self.state0())
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
    fn state0(&mut self) -> json::Value {
        let token = self.lex.token();
        let value = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => {
                let object = self.state5();
                self.state15(object)
            }
            Token { kind: TokenKind::LeftBracket, .. } => {
                let array = self.state16();
                self.state23(array)
            }
            _ => panic!("unexpected token {:?}", token),
        };
        value
    }

    /// S1 = value = STRING *
    fn state1(&mut self, string: String) -> json::Value {
        json::Value::String(string)
    }

    /// S2 = value = NUMBER *
    fn state2(&mut self, number: f64) -> json::Value {
        json::Value::Number(number)
    }

    /// S3 = value = BOOL *
    fn state3(&mut self, bool_: bool) -> json::Value {
        json::Value::Bool(bool_)
    }

    /// S4 = value = NULL *
    fn state4(&mut self) -> json::Value {
        json::Value::Null
    }

    /// S5 = object = '{' * pairs '}'
    ///      object = '{' * '}'
    ///      pairs = * pair
    ///      pairs = * pairs ',' pair
    ///      pair = * STRING ':' value
    fn state5(&mut self) -> json::Object {
        let token = self.lex.token();
        let pairs = match token {
            Token { kind: TokenKind::String(string), .. } => {
                let pair = self.state6(string);
                self.state9(pair)
            }
            Token { kind: TokenKind::RightBrace, .. } => return self.state14(),
            _ => panic!("unexpected token {:?}", token),
        };
        let mut nonterminal = Either::Left(pairs);
        loop {
            match nonterminal {
                Either::Left(object) => nonterminal = self.state10(object),
                Either::Right(object) => break object,
            }
        }
    }

    /// S6 = pair = STRING * ':' value
    fn state6(&mut self, string: String) -> (String, json::Value) {
        let token = self.lex.token();
        match token {
            Token { kind: TokenKind::Colon, .. } => self.state7(string),
            token => panic!("unexpected token {:?}", token),
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
    fn state7(&mut self, string: String) -> (String, json::Value) {
        let token = self.lex.token();
        let value = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => {
                let object = self.state5();
                self.state15(object)
            }
            Token { kind: TokenKind::LeftBracket, .. } => {
                let array = self.state16();
                self.state23(array)
            }
            token => panic!("unexpected token {:?}", token),
        };
        self.state8(string, value)
    }

    /// S8 = pair = STRING ':' value *
    fn state8(&mut self, string: String, value: json::Value) -> (String, json::Value) {
        (string, value)
    }

    /// S9 = pairs = pair *
    fn state9(&mut self, (string, value): (String, json::Value)) -> json::Object {
        let mut object = json::Object::new();
        object.insert(string, value);
        object
    }

    /// S10= object = '{' pairs * '}'
    ///      pairs = pairs * ',' pair
    fn state10(&mut self, object: json::Object) -> Either<json::Object, json::Object> {
        let token = self.lex.token();
        match token {
            Token { kind: TokenKind::Comma, .. } => Either::Left(self.state11(object)),
            Token { kind: TokenKind::RightBrace, .. } => Either::Right(self.state13(object)),
            token => panic!("unexpected token {:?}", token),
        }
    }

    /// S11= pairs = pairs ',' * pair
    ///      pair = * STRING ':' value
    fn state11(&mut self, object: json::Object) -> json::Object {
        let token = self.lex.token();
        let pair = match token {
            Token { kind: TokenKind::String(string), .. } => self.state6(string),
            token => panic!("unexpected token {:?}", token),
        };
        self.state12(object, pair)
    }

    /// S12= pairs = pairs ',' pair *
    fn state12(&mut self, object: json::Object, (string, value): (String, json::Value)) -> json::Object {
        let mut object = object;
        object.insert(string, value);
        object
    }

    /// S13= object = '{' pairs '}' *
    fn state13(&mut self, object: json::Object) -> json::Object {
        object
    }

    /// S14= object = '{' '}' *
    fn state14(&mut self) -> json::Object {
        json::Object::new()
    }

    /// S15= value = object *
    fn state15(&mut self, object: json::Object) -> json::Value {
        json::Value::Object(object)
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
    fn state16(&mut self) -> json::Array {
        let token = self.lex.token();
        let value = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => {
                let object = self.state5();
                self.state15(object)
            }
            Token { kind: TokenKind::LeftBracket, .. } => {
                let array = self.state16();
                self.state23(array)
            }
            Token { kind: TokenKind::RightBracket, .. } => return self.state22(),
            token => panic!("unexpected token {:?}", token),
        };
        let mut nonterminal = Either::Left(self.state17(value));
        loop {
            match nonterminal {
                Either::Left(array) => nonterminal = self.state18(array),
                Either::Right(array) => break array,
            }
        }
    }

    /// S17= elements = value *
    fn state17(&mut self, value: json::Value) -> json::Array {
        let mut array = json::Array::new();
        array.push(value);
        array
    }

    /// S18= array = '[' elements * ']'
    ///      elements = elements * ',' value
    fn state18(&mut self, array: json::Array) -> Either<json::Array, json::Array> {
        let token = self.lex.token();
        match token {
            Token { kind: TokenKind::Comma, .. } => Either::Left(self.state19(array)),
            Token { kind: TokenKind::RightBracket, .. } => Either::Right(self.state21(array)),
            token => panic!("unexpected token {:?}", token),
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
    fn state19(&mut self, array: json::Array) -> json::Array {
        let token = self.lex.token();
        let value = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => {
                let object = self.state5();
                self.state15(object)
            }
            Token { kind: TokenKind::LeftBracket, .. } => {
                let array = self.state16();
                self.state23(array)
            }
            token => panic!("unexpected token {:?}", token),
        };
        self.state20(array, value)
    }

    /// S20= elements = elements ',' value *
    fn state20(&mut self, array: json::Array, value: json::Value) -> json::Array {
        let mut array = array;
        array.push(value);
        array
    }

    /// S21= array = '[' elements ']' *
    fn state21(&mut self, array: json::Array) -> json::Array {
        array
    }

    /// S22= array = '[' ']' *
    fn state22(&mut self) -> json::Array {
        let array = json::Array::new();
        array
    }

    /// S23 = value = array *
    fn state23(&mut self, array: json::Array) -> json::Value {
        json::Value::Array(array)
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
