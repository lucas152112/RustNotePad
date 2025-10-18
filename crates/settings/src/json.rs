use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsonError {
    UnexpectedEnd,
    UnexpectedToken(char),
    InvalidNumber,
    InvalidEscape,
    Expected(&'static str),
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::UnexpectedEnd => write!(f, "unexpected end of input"),
            JsonError::UnexpectedToken(ch) => write!(f, "unexpected token '{ch}'"),
            JsonError::InvalidNumber => write!(f, "invalid number literal"),
            JsonError::InvalidEscape => write!(f, "invalid escape sequence"),
            JsonError::Expected(msg) => write!(f, "expected {msg}"),
        }
    }
}

impl std::error::Error for JsonError {}

pub fn parse(input: &str) -> Result<JsonValue, JsonError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.peek().is_some() {
        return Err(JsonError::UnexpectedToken(parser.peek().unwrap()));
    }
    Ok(value)
}

pub fn stringify_pretty(value: &JsonValue, indent: usize) -> String {
    let mut buf = String::new();
    write_pretty(value, indent, 0, &mut buf);
    buf
}

fn write_pretty(value: &JsonValue, indent: usize, level: usize, buf: &mut String) {
    match value {
        JsonValue::Null => buf.push_str("null"),
        JsonValue::Bool(b) => buf.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => {
            if (n.fract()).abs() < f64::EPSILON {
                buf.push_str(&format!("{}", *n as i64));
            } else {
                buf.push_str(&format!("{n}"));
            }
        }
        JsonValue::String(s) => {
            buf.push('"');
            for ch in s.chars() {
                match ch {
                    '\\' => buf.push_str("\\\\"),
                    '"' => buf.push_str("\\\""),
                    '\n' => buf.push_str("\\n"),
                    '\r' => buf.push_str("\\r"),
                    '\t' => buf.push_str("\\t"),
                    other => buf.push(other),
                }
            }
            buf.push('"');
        }
        JsonValue::Array(items) => {
            if items.is_empty() {
                buf.push_str("[]");
                return;
            }
            buf.push('[');
            buf.push('\n');
            for (index, item) in items.iter().enumerate() {
                buf.push_str(&" ".repeat(indent * (level + 1)));
                write_pretty(item, indent, level + 1, buf);
                if index + 1 != items.len() {
                    buf.push(',');
                }
                buf.push('\n');
            }
            buf.push_str(&" ".repeat(indent * level));
            buf.push(']');
        }
        JsonValue::Object(map) => {
            if map.is_empty() {
                buf.push_str("{}");
                return;
            }
            buf.push('{');
            buf.push('\n');
            let len = map.len();
            for (idx, (key, value)) in map.iter().enumerate() {
                buf.push_str(&" ".repeat(indent * (level + 1)));
                buf.push('"');
                buf.push_str(key);
                buf.push_str("\": ");
                write_pretty(value, indent, level + 1, buf);
                if idx + 1 != len {
                    buf.push(',');
                }
                buf.push('\n');
            }
            buf.push_str(&" ".repeat(indent * level));
            buf.push('}');
        }
    }
}

struct Parser<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            Some('n') => self.parse_literal("null", JsonValue::Null),
            Some('t') => self.parse_literal("true", JsonValue::Bool(true)),
            Some('f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some('\"') => self.parse_string().map(JsonValue::String),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_object(),
            Some('-') | Some('0'..='9') => self.parse_number(),
            Some(ch) => Err(JsonError::UnexpectedToken(ch)),
            None => Err(JsonError::UnexpectedEnd),
        }
    }

    fn parse_literal(
        &mut self,
        expected: &'static str,
        value: JsonValue,
    ) -> Result<JsonValue, JsonError> {
        for expected_char in expected.chars() {
            if self.next_char() != Some(expected_char) {
                return Err(JsonError::Expected(expected));
            }
        }
        Ok(value)
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect('\"')?;
        let mut result = String::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '\"' => return Ok(result),
                '\\' => {
                    let escaped = self.next_char().ok_or(JsonError::InvalidEscape)?;
                    let translated = match escaped {
                        '\"' => '\"',
                        '\\' => '\\',
                        '/' => '/',
                        'b' => '\u{0008}',
                        'f' => '\u{000C}',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        _ => return Err(JsonError::InvalidEscape),
                    };
                    result.push(translated);
                }
                other => result.push(other),
            }
        }
        Err(JsonError::UnexpectedEnd)
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.expect('[')?;
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.peek() == Some(']') {
            self.expect(']')?;
            return Ok(JsonValue::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.position += 1;
                    self.skip_whitespace();
                }
                Some(']') => {
                    self.position += 1;
                    break;
                }
                Some(ch) => return Err(JsonError::UnexpectedToken(ch)),
                None => return Err(JsonError::UnexpectedEnd),
            }
        }
        Ok(JsonValue::Array(items))
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.expect('{')?;
        self.skip_whitespace();
        let mut map = BTreeMap::new();
        if self.peek() == Some('}') {
            self.expect('}')?;
            return Ok(JsonValue::Object(map));
        }
        loop {
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(':')?;
            self.skip_whitespace();
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.position += 1;
                    self.skip_whitespace();
                }
                Some('}') => {
                    self.position += 1;
                    break;
                }
                Some(ch) => return Err(JsonError::UnexpectedToken(ch)),
                None => return Err(JsonError::UnexpectedEnd),
            }
        }
        Ok(JsonValue::Object(map))
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.position;
        if self.peek() == Some('-') {
            self.position += 1;
        }
        self.consume_digits();
        if self.peek() == Some('.') {
            self.position += 1;
            self.consume_digits();
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.position += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.position += 1;
            }
            self.consume_digits();
        }
        let slice = &self.input[start..self.position];
        let text = std::str::from_utf8(slice).map_err(|_| JsonError::InvalidNumber)?;
        if let Ok(num) = text.parse::<f64>() {
            Ok(JsonValue::Number(num))
        } else {
            Err(JsonError::InvalidNumber)
        }
    }

    fn consume_digits(&mut self) {
        while matches!(self.peek(), Some('0'..='9')) {
            self.position += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n' | '\r' | '\t')) {
            self.position += 1;
        }
    }

    fn expect(&mut self, ch: char) -> Result<(), JsonError> {
        match self.next_char() {
            Some(actual) if actual == ch => Ok(()),
            Some(actual) => Err(JsonError::UnexpectedToken(actual)),
            None => Err(JsonError::UnexpectedEnd),
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.position).map(|byte| *byte as char)
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some(&byte) = self.input.get(self.position) {
            self.position += 1;
            Some(byte as char)
        } else {
            None
        }
    }
}

impl JsonValue {
    pub fn as_object(&self) -> Result<&BTreeMap<String, JsonValue>, JsonError> {
        match self {
            JsonValue::Object(map) => Ok(map),
            _ => Err(JsonError::Expected("object")),
        }
    }

    pub fn as_array(&self) -> Result<&[JsonValue], JsonError> {
        match self {
            JsonValue::Array(values) => Ok(values),
            _ => Err(JsonError::Expected("array")),
        }
    }

    pub fn as_str(&self) -> Result<&str, JsonError> {
        match self {
            JsonValue::String(text) => Ok(text),
            _ => Err(JsonError::Expected("string")),
        }
    }

    pub fn as_f64(&self) -> Result<f64, JsonError> {
        match self {
            JsonValue::Number(num) => Ok(*num),
            _ => Err(JsonError::Expected("number")),
        }
    }

    pub fn as_bool(&self) -> Result<bool, JsonError> {
        match self {
            JsonValue::Bool(value) => Ok(*value),
            _ => Err(JsonError::Expected("bool")),
        }
    }
}
