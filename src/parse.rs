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
        match self.state0() {
            Nonterminal::Value(value) => Ok(value),
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
    fn state0(&mut self) -> Nonterminal {
        let token = self.lex.token();
        let mut nonterminal;
        nonterminal = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => self.state5(),
            Token { kind: TokenKind::LeftBracket, .. } => self.state16(),
            _ => panic!("unexpected token {:?}", token),
        };
        match nonterminal {
            Nonterminal::Object(object) => nonterminal = self.state15(object),
            Nonterminal::Array(array) => nonterminal = self.state23(array),
            _ => (),
        }
        nonterminal
    }

    /// S1 = value = STRING *
    fn state1(&mut self, string: String) -> Nonterminal {
        let value = json::Value::String(string);
        Nonterminal::Value(value)
    }

    /// S2 = value = NUMBER *
    fn state2(&mut self, number: f64) -> Nonterminal {
        let value = json::Value::Number(number);
        Nonterminal::Value(value)
    }

    /// S3 = value = BOOL *
    fn state3(&mut self, bool_: bool) -> Nonterminal {
        let value = json::Value::Bool(bool_);
        Nonterminal::Value(value)
    }

    /// S4 = value = NULL *
    fn state4(&mut self) -> Nonterminal {
        let value = json::Value::Null;
        Nonterminal::Value(value)
    }

    /// S5 = object = '{' * pairs '}'
    ///      object = '{' * '}'
    ///      pairs = * pair
    ///      pairs = * pairs ',' pair
    ///      pair = * STRING ':' value
    fn state5(&mut self) -> Nonterminal {
        let token = self.lex.token();
        let mut nonterminal;
        nonterminal = match token {
            Token { kind: TokenKind::String(string), .. } => self.state6(string),
            Token { kind: TokenKind::RightBrace, .. } => self.state14(),
            _ => panic!("unexpected token {:?}", token),
        };
        if let Nonterminal::Pair(pair) = nonterminal {
            nonterminal = self.state9(pair);
        }
        while let Nonterminal::Pairs(object) = nonterminal {
            nonterminal = self.state10(object);
        }
        nonterminal
    }

    /// S6 = pair = STRING * ':' value
    fn state6(&mut self, string: String) -> Nonterminal {
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
    fn state7(&mut self, string: String) -> Nonterminal {
        let token = self.lex.token();
        let mut nonterminal;
        nonterminal = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => self.state5(),
            Token { kind: TokenKind::LeftBracket, .. } => self.state16(),
            token => panic!("unexpected token {:?}", token),
        };
        match nonterminal {
            Nonterminal::Object(object) => nonterminal = self.state15(object),
            Nonterminal::Array(array) => nonterminal = self.state23(array),
            _ => (),
        }
        if let Nonterminal::Value(value) = nonterminal {
            nonterminal = self.state8(string, value);
        }
        nonterminal
    }

    /// S8 = pair = STRING ':' value *
    fn state8(&mut self, string: String, value: json::Value) -> Nonterminal {
        let pair = (string, value);
        Nonterminal::Pair(pair)
    }

    /// S9 = pairs = pair *
    fn state9(&mut self, (string, value): (String, json::Value)) -> Nonterminal {
        let mut object = json::Object::new();
        object.insert(string, value);
        Nonterminal::Pairs(object)
    }

    /// S10= object = '{' pairs * '}'
    ///      pairs = pairs * ',' pair
    fn state10(&mut self, object: json::Object) -> Nonterminal {
        let token = self.lex.token();
        match token {
            Token { kind: TokenKind::Comma, .. } => self.state11(object),
            Token { kind: TokenKind::RightBrace, .. } => self.state13(object),
            token => panic!("unexpected token {:?}", token),
        }
    }

    /// S11= pairs = pairs ',' * pair
    ///      pair = * STRING ':' value
    fn state11(&mut self, object: json::Object) -> Nonterminal {
        let token = self.lex.token();
        let mut nonterminal;
        nonterminal = match token {
            Token { kind: TokenKind::String(string), .. } => self.state6(string),
            token => panic!("unexpected token {:?}", token),
        };
        if let Nonterminal::Pair(pair) = nonterminal {
            nonterminal = self.state12(object, pair);
        }
        nonterminal
    }

    /// S12= pairs = pairs ',' pair *
    fn state12(&mut self, object: json::Object, (string, value): (String, json::Value)) -> Nonterminal {
        let mut object = object;
        object.insert(string, value);
        Nonterminal::Pairs(object)
    }

    /// S13= object = '{' pairs '}' *
    fn state13(&mut self, object: json::Object) -> Nonterminal {
        Nonterminal::Object(object)
    }

    /// S14= object = '{' '}' *
    fn state14(&mut self) -> Nonterminal {
        let object = json::Object::new();
        Nonterminal::Object(object)
    }

    /// S15= value = object *
    fn state15(&mut self, object: json::Object) -> Nonterminal {
        let value = json::Value::Object(object);
        Nonterminal::Value(value)
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
    fn state16(&mut self) -> Nonterminal {
        let token = self.lex.token();
        let mut nonterminal;
        nonterminal = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => self.state5(),
            Token { kind: TokenKind::LeftBracket, .. } => self.state16(),
            Token { kind: TokenKind::RightBracket, .. } => return self.state22(),
            token => panic!("unexpected token {:?}", token),
        };
        match nonterminal {
            Nonterminal::Object(object) => nonterminal = self.state15(object),
            Nonterminal::Array(array) => nonterminal = self.state23(array),
            _ => (),
        }
        if let Nonterminal::Value(value) = nonterminal {
            nonterminal = self.state17(value);
        }
        while let Nonterminal::Elements(array) = nonterminal {
            nonterminal = self.state18(array);
        }
        nonterminal
    }

    /// S17= elements = value *
    fn state17(&mut self, value: json::Value) -> Nonterminal {
        let mut array = json::Array::new();
        array.push(value);
        Nonterminal::Elements(array)
    }

    /// S18= array = '[' elements * ']'
    ///      elements = elements * ',' value
    fn state18(&mut self, array: json::Array) -> Nonterminal {
        let token = self.lex.token();
        match token {
            Token { kind: TokenKind::Comma, .. } => self.state19(array),
            Token { kind: TokenKind::RightBracket, .. } => { self.state21(array) },
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
    fn state19(&mut self, array: json::Array) -> Nonterminal {
        let token = self.lex.token();
        let mut nonterminal;
        nonterminal = match token {
            Token { kind: TokenKind::String(string), .. } => self.state1(string),
            Token { kind: TokenKind::Number(number), .. } => self.state2(number),
            Token { kind: TokenKind::Bool(bool_), .. } => self.state3(bool_),
            Token { kind: TokenKind::Null, .. } => self.state4(),
            Token { kind: TokenKind::LeftBrace, .. } => self.state5(),
            Token { kind: TokenKind::LeftBracket, .. } => self.state16(),
            token => panic!("unexpected token {:?}", token),
        };
        match nonterminal {
            Nonterminal::Object(object) => nonterminal = self.state15(object),
            Nonterminal::Array(array) => nonterminal = self.state23(array),
            _ => ()
        }
        if let Nonterminal::Value(value) = nonterminal {
            nonterminal = self.state20(array, value);
        }
        nonterminal
    }

    /// S20= elements = elements ',' value *
    fn state20(&mut self, array: json::Array, value: json::Value) -> Nonterminal {
        let mut array = array;
        array.push(value);
        Nonterminal::Elements(array)
    }

    /// S21= array = '[' elements ']' *
    fn state21(&mut self, array: json::Array) -> Nonterminal {
        Nonterminal::Array(array)
    }

    /// S22= array = '[' ']' *
    fn state22(&mut self) -> Nonterminal {
        let array = json::Array::new();
        Nonterminal::Array(array)
    }

    /// S23 = value = array *
    fn state23(&mut self, array: json::Array) -> Nonterminal {
        let value = json::Value::Array(array);
        Nonterminal::Value(value)
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
