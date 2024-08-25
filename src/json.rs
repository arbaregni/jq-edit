use anyhow::{
    bail, Result
};

use std::borrow::Cow;

use crate::tokens::{self, Token, TokenType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonData<'a>(JsonDataInner<'a>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonDataInner<'a> {
    ty: JsonDataType<'a>,
    lex: Cow<'a, str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonDataType<'a> {
    Object { elems: Vec<(JsonKey<'a>, JsonData<'a>)> },
    Array { elems: Vec<JsonData<'a>> },
    Str,
    Boolean,
    Number,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonKey<'a> {
    lex: Cow<'a, str>,
}


struct ParsingContext<'a> {
    source: &'a str,
    tokens: Vec<Token<'a>>,
    idx: usize,
    errs: Vec<String>,
}
impl <'a> ParsingContext<'a> {
    fn peek(&self) -> Token<'a> {
        match self.tokens.get(self.idx) {
            Some(tok) => *tok,
            None => Token {
                tty: TokenType::Eof,
                lex: "",
            }
        }
    }
    fn matches(&self, tty: TokenType) -> bool {
        self.peek().tty == tty
    }
    fn eat_whitespace(&mut self) {
        // we can ignore white space while parsing
        while self.peek().tty.is_whitespace() {
            self.idx += 1;
        }
    }
    fn consume(&mut self, tty: TokenType) -> Result<()> {
        self.eat_whitespace();
        if !self.matches(tty) {
            bail!("Expected {tty:?}, got {:?}", self.peek().tty);
        }
        self.idx += 1;
        Ok(())
    }
    fn parse_json(&mut self) -> Result<JsonData<'a>> {
        self.eat_whitespace();

        let tty = self.peek().tty;
        let json = match tty {
            TokenType::OpenBrace => self.parse_object()?,
            TokenType::OpenBracket => self.parse_array()?,
            TokenType::String => self.parse_single_token_as(JsonDataType::Str)?,
            TokenType::Number => self.parse_single_token_as(JsonDataType::Number)?,
            TokenType::Boolean => self.parse_single_token_as(JsonDataType::Boolean)?,
            _ => { 
                bail!("unexpected token type {tty:?}");
            }
        };
        Ok(json)
    }
    fn parse_object(&mut self) -> Result<JsonData<'a>> {
        bail!("todo");
    }
    fn parse_array(&mut self) -> Result<JsonData<'a>> {
        self.consume(TokenType::OpenBracket)?;

        let mut elems = Vec::new();

        loop {
            match self.parse_json() {
                Ok(elem) => {
                    elems.push(elem);
                }
                Err(err) => {
                    self.errs.push(err.to_string())
                }
            }

            self.eat_whitespace();

            match self.peek().tty {
                TokenType::Comma => {
                    // go around again (this is no trailing comma parsing)
                }
                TokenType::CloseBracket => {
                    // consume it and break
                    self.consume(TokenType::CloseBracket)?;
                    break;
                }
                _ => bail!("unexpected token while parsing array: {:?}", self.peek().tty)
            }
        }

        Ok(JsonData(JsonDataInner {
            ty: JsonDataType::Array {
                elems
            },
            lex: Cow::from("todo"),
       }))
    }
    fn parse_single_token_as(&mut self, ty: JsonDataType<'a>) -> Result<JsonData<'a>> {
        self.eat_whitespace();

        let tok = self.peek();
        let lex = Cow::Borrowed(tok.lex);
        Ok(JsonData(JsonDataInner {
            ty,
            lex
        }))
    }
}

pub fn loads<'a>(source: &'a str) -> Result<JsonData<'a>> {
    let tokens = tokens::tokenize(source);
    let mut ctx = ParsingContext {
        source,
        tokens,
        idx: 0,
        errs: vec![],
    };
    let json = ctx.parse_json()?;
    ctx.eat_whitespace();
    ctx.consume(TokenType::Eof)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int() {
        let source = "  \t\n 34";
        let expected = JsonData(JsonDataInner {
            ty: JsonDataType::Number,
            lex: Cow::from("34")
        });

        let actual = loads(source).expect("this should parse");

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_boolean() {
        let source = "true";
        let expected = JsonData(JsonDataInner {
            ty: JsonDataType::Boolean,
            lex: Cow::from("true")
        });

        let actual = loads(source).expect("this should parse");

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_string() {
        let source = " \"hello world\" ";
        let expected = JsonData(JsonDataInner {
            ty: JsonDataType::Str,
            lex: Cow::from("\"hello world\"")
        });

        let actual = loads(source).expect("this should parse");

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_array() {
        let source = "[34, true, \"hello world\"]";
        let expected = JsonData(JsonDataInner {
            ty: JsonDataType::Array {
                elems: vec![
                    JsonData(JsonDataInner {
                        ty: JsonDataType::Number,
                        lex: Cow::from("34"),
                    }),
                    JsonData(JsonDataInner {
                        ty: JsonDataType::Boolean,
                        lex: Cow::from("true"),
                    }),
                    JsonData(JsonDataInner {
                        ty: JsonDataType::Str,
                        lex: Cow::from("\"hello world\""),
                    }),
                ],
            },
            lex: Cow::from(source)
        });

        let actual = loads(source).expect("this should parse");

        assert_eq!(actual, expected);
    }

}

