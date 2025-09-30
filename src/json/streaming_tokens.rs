use buffered_reader::BufferedReader;
use tokens::TokenType;
use crate::streaming::Streaming;
use crate::json::*;

use anyhow::{bail, Result};

pub type CookieType = ();

const DATA_WINDOW: usize = 1024;

pub const TOKEN_LITERALS: [(TokenType, &'static str); 15] = [
    (TokenType::OpenBrace,      "{"),
    (TokenType::CloseBrace,     "}"),
    (TokenType::OpenBracket,    "["),
    (TokenType::CloseBracket,   "]"),
    (TokenType::Comma,          ","),
    (TokenType::Colon,          ":"),
    (TokenType::Newline,        "\n"),
    (TokenType::Whitespace,     "\r"),
    (TokenType::Whitespace,     "\t"),
    (TokenType::Whitespace,     " "),
    (TokenType::Boolean,        "true"),
    (TokenType::Boolean,        "false"),
    (TokenType::Number,         "inf"),
    (TokenType::Boolean,        "-inf"),
    (TokenType::Null,           "null"),
];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Token {
    pub tty: TokenType,
    pub lex: String,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: improve this
        write!(f, "{} {}", self.lex, self.tty)
    }
}

pub fn tok_to_num(token: &Token) -> Result<JsonData<'static>> {
    use IntOrFloat::*;

    let value = match token.lex.as_str() {
        "inf" => JsonData::Float { value: f64::INFINITY },
        "-inf" => JsonData::Float { value: f64::NEG_INFINITY },
        input => match parse_scientific_notation(input)? {
            Int(value) => JsonData::Number { value },
            Float(value) => JsonData::Float { value },
        }
    };
    Ok(value)
}

pub fn tok_to_bool(token: &Token) -> Result<JsonData<'static>> {
    let value = token.lex.as_str().parse()?;
    Ok(JsonData::Boolean { value })
}
pub fn tok_to_str(token: &Token) -> Result<JsonData<'static>> {
    let value = enquote::unescape(&token.lex, None)?;
    let value = Cow::Owned(value); 
    Ok(JsonData::Str { value })
}


enum IntOrFloat {
    Int(i64),
    Float(f64)
}
fn parse_scientific_notation(input: &str) -> Result<IntOrFloat> {
    let value = match input.split_once(&['e', 'E']) {
        Some((mantissa, exponent)) => {
            use IntOrFloat::*;

            let mantissa = parse_maybe_decimal(mantissa)?;
            let exponent: i32 = exponent.parse()?;
            match mantissa {
                Int(m) => {
                    let exp: u32 = exponent.abs().try_into()?;
                    if exponent >= 0 {
                        Int(m * 10i64.pow(exp))
                    } else if (m % 10i64.pow(exp)) == 0 {
                        Int(m / 10i64.pow(exp))
                    } else {
                        Float(m as f64 * 10f64.powi(exponent))
                    }
                }
                Float(m) => Float(m * 10f64.powi(exponent))
            }
        }
        None => parse_maybe_decimal(input)?,
    };
    Ok(value)
}

fn parse_maybe_decimal(input: &str) -> Result<IntOrFloat> {
    if input.contains(".") {
        let value = input.parse()?;
        Ok(IntOrFloat::Float(value))
    } else {
        let value = input.parse()?;
        Ok(IntOrFloat::Int(value))
    }

}


pub struct StreamingTokens<BR> where BR: BufferedReader<CookieType> {
    reader: BR,
    byte_index: usize
}

impl <BR> StreamingTokens<BR> where BR: BufferedReader<CookieType> {
    /// Create a stream of tokens from a buffered reader
    pub fn from(reader: BR) -> Self {
        Self {
            reader,
            byte_index: 0
        }
    }
    fn advance(&mut self, amount: usize) {
        self.reader.consume(amount);
        self.byte_index += amount;
    }

    fn consume_if_startswith(&mut self, prefix: &str) -> Result<bool> {
        let prefix_bytes = prefix.as_bytes();
        let n = prefix_bytes.len();
        let data = self.reader.data(n)?;
        if data.starts_with(prefix_bytes) {
            self.advance(n);
            return Ok(true)
        }
        Ok(false)
    }

    fn data_as_utf8(&mut self, amount: usize) -> Result<Cow<'_, str>> {
        let data = self.reader.data(amount)?;
        Ok(String::from_utf8_lossy(data))
    }

    fn peek_char(&mut self) -> Result<Option<char>> {
        let data = self.data_as_utf8(DATA_WINDOW)?;
        Ok(data.chars().nth(0))
    }

    fn peek_if_char_matches<F: FnOnce(char) -> bool>(&mut self, pred: F) -> Result<Option<char>> {
        let Some(ch) = self.peek_char()? else {
            return Ok(None);
        };
        if pred(ch) {
            Ok(Some(ch))
        } else {
            Ok(None)
        }
    }
    fn consume_if_char_matches<F: FnOnce(char) -> bool>(&mut self,  buf: &mut String, pred: F) -> Result<Option<char>> {
        let Some(ch) = self.peek_char()? else {
            return Ok(None);
        };
        if pred(ch) {
            buf.push(ch);
            self.advance(ch.len_utf8());
            Ok(Some(ch))
        } else {
            Ok(None)
        }
    }


    fn consume_number(&mut self) -> Result<String> {
        let mut buf = String::with_capacity(10);
        let mut found_period = false;

        self.consume_if_char_matches(&mut buf, |ch| ch == '-')?;

        while let Some(ch) = self.consume_if_char_matches(&mut buf, |ch| ch.is_ascii_digit() || (!found_period && ch == '.'))? {
            if ch == '.' {
                found_period = true;
            }
        }

        if let Some(_) = self.consume_if_char_matches(&mut buf, |ch| ch == 'e' || ch == 'E')? {

            self.consume_if_char_matches(&mut buf, |ch| ch == '+' || ch == '-')?;

            while let Some(_) = self.consume_if_char_matches(&mut buf, |ch| ch.is_ascii_digit())? { }
        }


        Ok(buf)
    }

    fn consume_string(&mut self) -> Result<String> {
        let mut buf = String::with_capacity(10);

        if !self.consume_if_startswith("\"")? {
            bail!("bad start to string");
        }

        while let Some(ch) = self.consume_if_char_matches(&mut buf, |ch| ch != '"')? {

            if ch == '\\' {
                let Some(escaped_char) = self.peek_char()? else {
                    bail!("unterminated string");
                };
                buf.push(escaped_char);
                self.advance(escaped_char.len_utf8());
            }
        }

        if !self.consume_if_startswith("\"")? {
            bail!("unterminated string");
        }

        Ok(buf)
    }
}

impl <BR> Streaming for StreamingTokens<BR> where BR: BufferedReader<CookieType> {
    type Item = Token;

    fn try_next(&mut self) -> Result<Option<Token>> {
       if self.reader.eof() {
            return Ok(None);
        }

        // NOTE: issue with this is that it potentially can't find really long matches 
        for (tty, literal) in TOKEN_LITERALS.iter() {

            if self.consume_if_startswith(literal)? {
                let lex = literal.to_string();
                return Ok(Some(Token {
                    tty: *tty,
                    lex
                }))
            }
        }
        if let Some(_) = self.peek_if_char_matches(|ch| ch.is_ascii_digit() || ch == '.' || ch == '-')? {
            let lex = self.consume_number()?;
            return Ok(Some(Token {
                tty: TokenType::Number,
                lex
            }));
        }

        if let Some(_) = self.peek_if_char_matches(|ch| ch == '"')? {
            let lex = self.consume_string()?;
            return Ok(Some(Token {
                tty: TokenType::String,
                lex
            }));
        }


        // yield some invalid characters
        let Some(invalid_ch) = self.peek_char()? else {
            return Ok(None); // no more characters
        };
        let lex = String::from(invalid_ch);
        return Ok(Some(Token {
            tty: TokenType::InvalidChar,
            lex
        }));
    } 
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stream(test_string: &str) -> StreamingTokens<impl BufferedReader<CookieType> + '_> {
        let br = buffered_reader::Memory::new(test_string.as_bytes());
        StreamingTokens::from(br)
    }

    #[test]
    fn test_simple() {
        let mut s = make_stream("{}[],:\n\r \t");
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::OpenBrace, lex: "{".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::CloseBrace, lex: "}".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::OpenBracket, lex: "[".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::CloseBracket, lex: "]".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Comma, lex: ",".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Colon, lex: ":".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Newline, lex: "\n".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Whitespace, lex: "\r".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Whitespace, lex: " ".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Whitespace, lex: "\t".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }

    #[test]
    fn test_whole_numbers() {
        let mut s = make_stream("1234567890");
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Number, lex: "1234567890".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }

    #[test]
    fn test_mixed_numbers() {
        let mut s = make_stream("[1234]");
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::OpenBracket, lex: "[".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Number, lex: "1234".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::CloseBracket, lex: "]".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }

    #[test]
    fn test_decimal() {
        let mut s = make_stream("123.456");
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Number, lex: "123.456".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }

    #[test]
    fn test_decimal2() {
        let mut s = make_stream(".456");
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Number, lex: ".456".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }

    #[test]
    fn test_decimal3() {
        let mut s = make_stream("123.");
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Number, lex: "123.".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }

    #[test]
    fn test_negative_decimal() {
        let mut s = make_stream("-12.3");
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Number, lex: "-12.3".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }

    #[test]
    fn test_scientific1() {
        let mut s = make_stream("4e10");
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Number, lex: "4e10".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }

   #[test]
    fn test_scientific_negative_exp() {
        let mut s = make_stream("4e-10");
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::Number, lex: "4e-10".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }


    #[test]
    fn test_string_simple() {
        let mut s = make_stream(r#""hello""#);
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::String, lex: "hello".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }

    #[test]
    fn test_string_quoted() {
        let mut s = make_stream(r#""he\"llo""#);
        assert_eq!(s.try_next().expect("succeed"), Some(Token { tty: TokenType::String, lex: "he\\\"llo".to_string() }));
        assert_eq!(s.try_next().expect("succeed"), None);
    }



}
