use lex::{Lex, Token, TokenKind};
use json;

/// A JSON parser.
pub struct Parse<'source> {
    lex: Lex<'source>,
}

/// Nonterminals for a JSON grammar.
///
/// The payloads are the results of the productions' actions.
///
/// value = STRING | NUMBER | BOOL | NULL | object | array
///
/// object = '{' pairs '}' | '{' '}'
/// pairs = pair | pairs ',' pair
/// pair = STRING ':' value
///
/// array = '[' elements ']' | '[' ']'
/// elements = value | elements ',' value
enum Nonterminal {
    Value(json::Value),
    Object(json::Object),
    Pairs(json::Object),
    Pair((String, json::Value)),
    Array(json::Array),
    Elements(json::Array),
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
        let token = self.lex.token();
        match self.state0(token) {
            (Token { kind: TokenKind::End, .. }, Nonterminal::Value(value)) => Ok(value),
            _ => Err(()),
        }
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
    fn state0(&mut self, token: Token<'source>) -> (Token<'source>, Nonterminal) {
        let mut result;
        result = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => self.state5(),
            Token { kind: TokenKind::LeftBracket, .. } => self.state16(&mut Some(())),
            _ => panic!("unexpected token {:?}", token),
        };
        loop {
            let (token, nonterminal) = result;
            match nonterminal {
                Nonterminal::Object(object) => result = self.state15(token, object),
                Nonterminal::Array(array) => result = self.state23(token, array),
                nonterminal => break (token, nonterminal),
            }
        }
    }

    /// S1 = value = STRING *
    fn state1(&mut self, string: String) -> (Token<'source>, Nonterminal) {
        let token = self.lex.token();
        let value = json::Value::String(string);
        let nonterminal = Nonterminal::Value(value);
        (token, nonterminal)
    }

    /// S2 = value = NUMBER *
    fn state2(&mut self, number: f64) -> (Token<'source>, Nonterminal) {
        let token = self.lex.token();
        let value = json::Value::Number(number);
        let nonterminal = Nonterminal::Value(value);
        (token, nonterminal)
    }

    /// S3 = value = BOOL *
    fn state3(&mut self, bool_: bool) -> (Token<'source>, Nonterminal) {
        let token = self.lex.token();
        let value = json::Value::Bool(bool_);
        let nonterminal = Nonterminal::Value(value);
        (token, nonterminal)
    }

    /// S4 = value = NULL *
    fn state4(&mut self) -> (Token<'source>, Nonterminal) {
        let token = self.lex.token();
        let value = json::Value::Null;
        let nonterminal = Nonterminal::Value(value);
        (token, nonterminal)
    }

    /// S5 = object = '{' * pairs '}'
    ///      object = '{' * '}'
    ///      pairs = * pair
    ///      pairs = * pairs ',' pair
    ///      pair = * STRING ':' value
    fn state5(&mut self) -> (Token<'source>, Nonterminal) {
        let mut result;
        let token = self.lex.token();
        result = match token {
            Token { kind: TokenKind::String(string), .. } => self.state6(string),
            Token { kind: TokenKind::RightBrace, .. } => self.state14(),
            _ => panic!("unexpected token {:?}", token),
        };
        loop {
            let (token, nonterminal) = result;
            match nonterminal {
                Nonterminal::Pair(pair) => result = self.state9(token, pair),
                Nonterminal::Pairs(object) => result = self.state10(token, object),
                nonterminal => break (token, nonterminal),
            }
        }
    }

    /// S6 = pair = STRING * ':' value
    fn state6(&mut self, string: String) -> (Token<'source>, Nonterminal) {
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
    fn state7(&mut self, string: String) -> (Token<'source>, Nonterminal) {
        let mut result;
        let token = self.lex.token();
        result = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => self.state5(),
            Token { kind: TokenKind::LeftBracket, .. } => self.state16(&mut Some(())),
            token => panic!("unexpected token {:?}", token),
        };
        loop {
            let (token, nonterminal) = result;
            match nonterminal {
                Nonterminal::Object(object) => result = self.state15(token, object),
                Nonterminal::Array(array) => result = self.state23(token, array),
                Nonterminal::Value(value) => break self.state8(token, string, value),
                nonterminal => break (token, nonterminal),
            }
        }
    }

    /// S8 = pair = STRING ':' value *
    fn state8(&mut self, token: Token<'source>, string: String, value: json::Value) -> (Token<'source>, Nonterminal) {
        let pair = (string, value);
        let nonterminal = Nonterminal::Pair(pair);
        (token, nonterminal)
    }

    /// S9 = pairs = pair *
    fn state9(&mut self, token: Token<'source>, (string, value): (String, json::Value)) -> (Token<'source>, Nonterminal) {
        let mut object = json::Object::new();
        object.insert(string, value);
        let nonterminal = Nonterminal::Pairs(object);
        (token, nonterminal)
    }

    /// S10= object = '{' pairs * '}'
    ///      pairs = pairs * ',' pair
    fn state10(&mut self, token: Token<'source>, object: json::Object) -> (Token<'source>, Nonterminal) {
        match token {
            Token { kind: TokenKind::Comma, .. } => self.state11(object),
            Token { kind: TokenKind::RightBrace, .. } => self.state13(object),
            token => panic!("unexpected token {:?}", token),
        }
    }

    /// S11= pairs = pairs ',' * pair
    ///      pair = * STRING ':' value
    fn state11(&mut self, object: json::Object) -> (Token<'source>, Nonterminal) {
        let result;
        let token = self.lex.token();
        result = match token {
            Token { kind: TokenKind::String(string), .. } => self.state6(string),
            token => panic!("unexpected token {:?}", token),
        };
        loop {
            let (token, nonterminal) = result;
            match nonterminal {
                Nonterminal::Pair(pair) => break self.state12(token, object, pair),
                nonterminal => break (token, nonterminal),
            }
        }
    }

    /// S12= pairs = pairs ',' pair *
    fn state12(&mut self, token: Token<'source>, object: json::Object, (string, value): (String, json::Value)) -> (Token<'source>, Nonterminal) {
        let mut object = object;
        object.insert(string, value);
        let nonterminal = Nonterminal::Pairs(object);
        (token, nonterminal)
    }

    /// S13= object = '{' pairs '}' *
    fn state13(&mut self, object: json::Object) -> (Token<'source>, Nonterminal) {
        let token = self.lex.token();
        let nonterminal = Nonterminal::Object(object);
        (token, nonterminal)
    }

    /// S14= object = '{' '}' *
    fn state14(&mut self) -> (Token<'source>, Nonterminal) {
        let token = self.lex.token();
        let object = json::Object::new();
        let nonterminal = Nonterminal::Object(object);
        (token, nonterminal)
    }

    /// S15= value = object *
    fn state15(&mut self, token: Token<'source>, object: json::Object) -> (Token<'source>, Nonterminal) {
        let value = json::Value::Object(object);
        let nonterminal = Nonterminal::Value(value);
        (token, nonterminal)
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
    fn state16(&mut self, left_bracket: &mut Option<()>) -> (Token<'source>, Nonterminal) {
        let mut result;
        let token = self.lex.token();
        result = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => self.state5(),
            Token { kind: TokenKind::LeftBracket, .. } => self.state16(&mut Some(())),
            Token { kind: TokenKind::RightBracket, .. } => self.state22(left_bracket),
            token => panic!("unexpected token {:?}", token),
        };
        loop {
            if left_bracket.is_none() {
                break result;
            }
            let (token, nonterminal) = result;
            match nonterminal {
                Nonterminal::Object(object) => result = self.state15(token, object),
                Nonterminal::Array(array) => result = self.state23(token, array),
                Nonterminal::Value(value) => result = self.state17(token, value),
                Nonterminal::Elements(array) => result = self.state18(token, left_bracket, array),
                nonterminal => break (token, nonterminal),
            }
        }
    }

    /// S17= elements = value *
    fn state17(&mut self, token: Token<'source>, value: json::Value) -> (Token<'source>, Nonterminal) {
        let mut array = json::Array::new();
        array.push(value);
        let nonterminal = Nonterminal::Elements(array);
        (token, nonterminal)
    }

    /// S18= array = '[' elements * ']'
    ///      elements = elements * ',' value
    fn state18(&mut self, token: Token<'source>, left_bracket: &mut Option<()>, array: json::Array) -> (Token<'source>, Nonterminal) {
        match token {
            Token { kind: TokenKind::Comma, .. } => self.state19(array),
            Token { kind: TokenKind::RightBracket, .. } => { self.state21(left_bracket, array) },
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
    fn state19(&mut self, array: json::Array) -> (Token<'source>, Nonterminal) {
        let mut result;
        let token = self.lex.token();
        result = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => self.state5(),
            Token { kind: TokenKind::LeftBracket, .. } => self.state16(&mut Some(())),
            token => panic!("unexpected token {:?}", token),
        };
        loop {
            let (token, nonterminal) = result;
            match nonterminal {
                Nonterminal::Object(object) => result = self.state15(token, object),
                Nonterminal::Array(array) => result = self.state23(token, array),
                Nonterminal::Value(value) => break self.state20(token, array, value),
                nonterminal => break (token, nonterminal),
            }
        }
    }

    /// S20= elements = elements ',' value *
    fn state20(&mut self, token: Token<'source>, array: json::Array, value: json::Value) -> (Token<'source>, Nonterminal) {
        let mut array = array;
        array.push(value);
        let nonterminal = Nonterminal::Elements(array);
        (token, nonterminal)
    }

    /// S21= array = '[' elements ']' *
    fn state21(&mut self, left_bracket: &mut Option<()>, array: json::Array) -> (Token<'source>, Nonterminal) {
        let token = self.lex.token();
        left_bracket.take().unwrap();
        let nonterminal = Nonterminal::Array(array);
        (token, nonterminal)
    }

    /// S22= array = '[' ']' *
    fn state22(&mut self, left_bracket: &mut Option<()>) -> (Token<'source>, Nonterminal) {
        let token = self.lex.token();
        left_bracket.take().unwrap();
        let array = json::Array::new();
        let nonterminal = Nonterminal::Array(array);
        (token, nonterminal)
    }

    /// S23 = value = array *
    fn state23(&mut self, token: Token<'source>, array: json::Array) -> (Token<'source>, Nonterminal) {
        let value = json::Value::Array(array);
        let nonterminal = Nonterminal::Value(value);
        (token, nonterminal)
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
