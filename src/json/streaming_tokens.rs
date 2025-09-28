use buffered_reader::BufferedReader;
use tokens::TokenType;
use crate::streaming::Streaming;
use crate::json::*;

use anyhow::{bail, Result};

pub type CookieType = ();

const DATA_WINDOW: usize = 1024;

pub const TOKEN_LITERALS: [(TokenType, &'static str); 13] = [
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
    (TokenType::Null,           "null"),
];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Token {
    pub tty: TokenType,
    pub lex: String,
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

    fn consume_number(&mut self) -> Result<String> {
        let mut buf = String::with_capacity(10);
        let mut found_period = false;

        if let Some(ch) = self.peek_if_char_matches(|ch| ch == '-')? {
            buf.push(ch);
            self.advance(ch.len_utf8());
        }

        while let Some(ch) = self.peek_if_char_matches(|ch| ch.is_ascii_digit() || (!found_period && ch == '.'))? {
            buf.push(ch);
            self.advance(ch.len_utf8());
            if ch == '.' {
                found_period = true;
            }
        }

        Ok(buf)
    }

    fn consume_string(&mut self) -> Result<String> {
        let mut buf = String::with_capacity(10);

        if !self.consume_if_startswith("\"")? {
            bail!("bad start to string");
        }

        while let Some(ch) = self.peek_if_char_matches(|ch| ch != '"')? {
            buf.push(ch);
            self.advance(ch.len_utf8());

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
