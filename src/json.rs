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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonDataType<'a> {
    Object { entries: Vec<(JsonKey<'a>, JsonData<'a>)> },
    Array { elems: Vec<JsonData<'a>> },
    Str { lex: Cow<'a, str> },
    Boolean { lex: Cow<'a, str> },
    Number { lex: Cow<'a, str> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonKey<'a> {
    lex: Cow<'a, str>,
}


struct ParsingContext<'a> {
    _source: &'a str,
    tokens: Vec<Token<'a>>,
    idx: usize,
    errs: Vec<String>,
}
impl <'a> ParsingContext<'a> {
    fn from(source: &'a str, tokens: Vec<Token<'a>>) -> Self {
        Self {
            _source: source,
            tokens,
            idx: 0,
            errs: vec![],
        }
    }
    fn peek(&self) -> Token<'a> {
        match self.tokens.get(self.idx) {
            Some(tok) => *tok,
            None => Token {
                tty: TokenType::Eof,
                lex: "",
            }
        }
    }
    fn eat_whitespace(&mut self) {
        // we can ignore white space while parsing
        while self.peek().tty.is_whitespace() {
            log::trace!("consumed: {:?}", self.peek());
            self.idx += 1;
        }
    }
    fn consume(&mut self, tty: TokenType) -> Result<Token<'a>> {
        self.eat_whitespace();
        let tok = self.peek();
        if tok.tty != tty {
            bail!("Expected {tty:?}, got {:?}", self.peek().tty);
        }
        log::trace!("consumed: {:?}", self.peek());
        self.idx += 1;
        Ok(tok)
    }
    fn parse_json(&mut self) -> Result<JsonData<'a>> {
        self.eat_whitespace();

        log::debug!("begin parsing json");

        let tty = self.peek().tty;
        let json = match tty {
            TokenType::OpenBrace => self.parse_object()?,
            TokenType::OpenBracket => self.parse_array()?,
            TokenType::String => self.parse_string()?,
            TokenType::Number => self.parse_number()?,
            TokenType::Boolean => self.parse_boolean()?,
            _ => { 
                bail!("unexpected token type {tty:?}");
            }
        };
        Ok(json)
    }
    fn parse_object(&mut self) -> Result<JsonData<'a>> {
        self.consume(TokenType::OpenBrace)?;

        log::debug!("begin parsing object");

        let mut entries = Vec::new();

        loop {
            let key = self.parse_string()?;
            let _ = self.consume(TokenType::Colon)?;
            let value = self.parse_json()?;

            let JsonDataType::Str { lex } = key.0.ty else {
                panic!("self.parse_string() should return a JsonDataType::Str");
            };

            let key = JsonKey { lex };

            entries.push((key, value));

            self.eat_whitespace();
            match self.peek().tty {
                TokenType::Comma => {
                    self.consume(TokenType::Comma)?;
                    // go around again (no trailing comma allowed !)
                }
                TokenType::CloseBrace => {
                    self.consume(TokenType::CloseBrace)?;
                    break;
                }
                _ => {
                    bail!("unexpected token while parsing object: {:?}", self.peek().tty);
                }
            }
        }

        Ok(JsonData(JsonDataInner {
            ty: JsonDataType::Object {
                entries
            },
        }))

    }
    fn parse_array(&mut self) -> Result<JsonData<'a>> {
        self.consume(TokenType::OpenBracket)?;

        log::debug!("begin parsing array");

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
                    self.consume(TokenType::Comma)?;
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
       }))
    }
    fn parse_string(&mut self) -> Result<JsonData<'a>> {
        self.eat_whitespace();

        log::debug!("begin parsing string");

        let tok = self.consume(TokenType::String)?;
        let lex = Cow::Borrowed(tok.lex);
        Ok(JsonData(JsonDataInner {
            ty: JsonDataType::Str {
                lex
            }
        }))
    }
    fn parse_number(&mut self) -> Result<JsonData<'a>> {
        self.eat_whitespace();

        log::debug!("begin parsing number");

        let tok = self.consume(TokenType::Number)?;
        let lex = Cow::Borrowed(tok.lex);
        Ok(JsonData(JsonDataInner {
            ty: JsonDataType::Number {
                lex
            }
        }))
    }
    fn parse_boolean(&mut self) -> Result<JsonData<'a>> {
        self.eat_whitespace();

        log::debug!("begin parsing boolean");

        let tok = self.consume(TokenType::Boolean)?;
        let lex = Cow::Borrowed(tok.lex);
        Ok(JsonData(JsonDataInner {
            ty: JsonDataType::Boolean {
                lex
            }
        }))
    }

}

pub fn loads<'a>(source: &'a str) -> Result<JsonData<'a>> {
    let tokens = tokens::tokenize(source);
    let mut ctx = ParsingContext::from(source, tokens);
    let json = ctx.parse_json()?;
    ctx.eat_whitespace();
    ctx.consume(TokenType::Eof)?;
    Ok(json)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peek() {
        let source = "blah";
        let tokens = vec![
            Token {
                tty: TokenType::Number,
                lex: "34",
            },
            Token {
                tty: TokenType::String,
                lex: "\"hello\""
            }
        ];
        let ctx = ParsingContext::from(source, tokens); 

        assert_eq!(ctx.peek(), Token { 
            tty: TokenType::Number,
            lex: "34"
        });
    }

    #[test]
    fn test_peek_empty() {
        let source = "blah";
        let tokens = vec![
        ];
        let ctx = ParsingContext::from(source, tokens); 

        assert_eq!(ctx.peek(), Token { 
            tty: TokenType::Eof,
            lex: ""
        });
    }

    #[test]
    fn test_eat_whitespace() {
        let source = "blah";
        let tokens = vec![
            Token {
                tty: TokenType::Whitespace,
                lex: " \t",
            },
            Token {
                tty: TokenType::Newline,
                lex: "\n",
            },
            Token {
                tty: TokenType::Whitespace,
                lex: " ",
            },
            Token {
                tty: TokenType::Number,
                lex: "34",
            }
        ];
        let mut ctx = ParsingContext::from(source, tokens);

        assert_eq!(ctx.peek(), Token {
            tty: TokenType::Whitespace,
            lex: " \t",
        });

        ctx.eat_whitespace();

        assert_eq!(ctx.peek(), Token {
            tty: TokenType::Number,
            lex: "34"
        });
    }   

    #[test]
    fn test_consume_when_matches() {
        let source = "blah";
        let tokens = vec![
            Token {
                tty: TokenType::Number,
                lex: "34",
            }
        ];
        let mut ctx = ParsingContext::from(source, tokens);

        assert_eq!(ctx.peek(), Token {
            tty: TokenType::Number,
            lex: "34",
        });

        let tok = ctx.consume(TokenType::Number).expect("should find Number token");
        assert_eq!(tok, Token {
            tty: TokenType::Number,
            lex: "34"
        });

        assert_eq!(ctx.peek(), Token {
            tty: TokenType::Eof,
            lex: "",
        });

    }

    #[test]
    fn test_consume_when_does_not_match() {
        let source = "blah";
        let tokens = vec![
            Token {
                tty: TokenType::Number,
                lex: "34",
            }
        ];
        let mut ctx = ParsingContext::from(source, tokens);

        assert_eq!(ctx.peek(), Token {
            tty: TokenType::Number,
            lex: "34",
        });

        let result = ctx.consume(TokenType::String); 
        assert!(result.is_err());

        assert_eq!(ctx.peek(), Token {
            tty: TokenType::Number,
            lex: "34",
        });
    }


}

#[cfg(test)]
mod end_user_tests {
    use super::*;

    #[test]
    fn parse_int() {
        let source = "  \t\n 34";
        let expected = JsonData(JsonDataInner {
            ty: JsonDataType::Number { 
                lex: Cow::from("34")
            }
        });

        let actual = loads(source).expect("this should parse");

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_boolean() {
        let source = "true";
        let expected = JsonData(JsonDataInner {
            ty: JsonDataType::Boolean { 
                lex: Cow::from("true")
            }
        });

        let actual = loads(source).expect("this should parse");

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_string() {
        let source = " \"hello world\" ";
        let expected = JsonData(JsonDataInner {
            ty: JsonDataType::Str { 
                lex: Cow::from("\"hello world\"")
            }
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
                        ty: JsonDataType::Number { 
                            lex: Cow::from("34"),
                        }
                    }),
                    JsonData(JsonDataInner {
                        ty: JsonDataType::Boolean { 
                            lex: Cow::from("true"),
                        }
                    }),
                    JsonData(JsonDataInner {
                        ty: JsonDataType::Str { 
                            lex: Cow::from("\"hello world\""),
                        }
                    }),
                ],
            },
        });

        let actual = loads(source).expect("this should parse");

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_object_simple() {
        let source = "{ \"foo\": \"bar\" }";
        let expected = JsonData(JsonDataInner {
            ty: JsonDataType::Object {
                entries: vec![
                    (
                        JsonKey { lex: Cow::from("\"foo\"") },
                        JsonData(JsonDataInner {
                            ty: JsonDataType::Str { 
                                lex: Cow::from("\"bar\"")
                            }
                        })
                    )
                ]
            },
        });
         
        let actual = loads(source).expect("this should parse");

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_object_nested() {
        let source = "{ \"foo\": [34, true, \"hello world\", { \"a\" : \"b\", \"c\": \"d\" } ] }";
        let expected = JsonData(JsonDataInner {
            ty: JsonDataType::Object {
                entries: vec![
                    (
                        JsonKey { lex: Cow::from("\"foo\"") },
                        JsonData(JsonDataInner {
                            ty: JsonDataType::Array {
                                elems: vec![
                                    JsonData(JsonDataInner {
                                        ty: JsonDataType::Number {
                                            lex: Cow::from("34")
                                        },
                                    }),
                                    JsonData(JsonDataInner {
                                        ty: JsonDataType::Boolean {
                                            lex: Cow::from("true")
                                        },
                                    }),
                                    JsonData(JsonDataInner {
                                        ty: JsonDataType::Str {
                                            lex: Cow::from("\"hello world\"")
                                        },
                                    }),
                                    JsonData(JsonDataInner {
                                        ty: JsonDataType::Object {
                                            entries: vec![
                                                (
                                                    JsonKey { lex: Cow::from("\"a\"") },
                                                    JsonData(JsonDataInner {
                                                        ty: JsonDataType::Str {
                                                            lex: Cow::from("\"b\"")
                                                        },
                                                    }),
                                                ),
                                                (
                                                    JsonKey { lex: Cow::from("\"c\"") },
                                                    JsonData(JsonDataInner {
                                                        ty: JsonDataType::Str {
                                                            lex: Cow::from("\"d\"")
                                                        },
                                                    }),
                                                )
                                            ]
                                        },
                                    })
                                ]
                            }
                        })
                    )
                ]
            },
        });
         
        let actual = loads(source).expect("this should parse");

        assert_eq!(actual, expected);
    }


}

