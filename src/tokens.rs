use regex::Regex;
use once_cell::sync::Lazy;

static PATTERN_TOKENS: Lazy<Vec<(TokenType, Regex)>> = Lazy::new(|| {
    vec![
        (TokenType::OpenBrace, Regex::new(r"^\{").expect("compile regex")),
        (TokenType::CloseBrace, Regex::new(r"^\}").expect("compile regex")),
        (TokenType::OpenBracket, Regex::new(r"^\[").expect("compile regex")),
        (TokenType::CloseBracket, Regex::new(r"^\]").expect("compile regex")),
        (TokenType::Comma, Regex::new(r"^,").expect("compile regex")),
        (TokenType::Colon, Regex::new(r"^:").expect("compile regex")),
        (TokenType::Newline, Regex::new(r"^\r?\n").expect("compile regex")),
        (TokenType::Whitespace, Regex::new(r"^[ \t]+").expect("compile regex")),
        (TokenType::Boolean, Regex::new(r"^true|false").expect("compile regex")),
        (TokenType::String, Regex::new(r#"^"(?:[^"\\]|\\.)*""#).expect("compile regex")),
        // Splitting up the numbers into 3 patterns for clarity
        (TokenType::Number, Regex::new(r"^-?\d*\.\d+").expect("compile regex")),
        (TokenType::Number, Regex::new(r"^-?\d+\.\d*").expect("compile regex")),
        (TokenType::Number, Regex::new(r"^-?\d+").expect("compile regex")),
    ]
});


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TokenType {
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Comma,
    Colon,
    Whitespace,
    Newline,
    String,
    Number,
    Boolean,
    InvalidChar,
    Eof,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub tty: TokenType,
    pub lex: &'a str,
}

struct TokenizeContext<'a> {
    tokens: Vec<Token<'a>>,
    source: &'a str,
}
impl <'a> TokenizeContext<'a> {
    fn from(source: &'a str) -> Self {
        Self {
            tokens: Vec::new(),
            source
        }
    }
}

impl TokenType {
    pub fn is_whitespace(self) -> bool {
        match self {
            TokenType::Whitespace | TokenType::Newline => true,
            _ => false,
        }

    }
}

pub fn tokenize(source: &str) -> Vec<Token> {
    let mut ctx = TokenizeContext::from(source);
    while ctx.source.len() > 0 {

        let tok = PATTERN_TOKENS
            .iter()
            .find_map(|(tty, re)| {
                log::debug!("running {re:?}.find({:?})", ctx.source);
                re.find(ctx.source)
                    .map(|capt| Token {
                        tty: tty.clone(),
                        lex: capt.as_str()
                    })
            })
            .unwrap_or(Token {
                tty: TokenType::InvalidChar,
                lex: &ctx.source[0..1],
            });

        let n = tok.lex.len();
        ctx.source = &ctx.source[n..];
        ctx.tokens.push(tok);

    }
    ctx.tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_punctuation() {
        let source = "{}[],:";
        let tokens = tokenize(source);

        assert_eq!(tokens.len(), 6);
        assert_eq!(&tokens[0], &Token {
            tty: TokenType::OpenBrace,
            lex: "{",
        });
        assert_eq!(&tokens[1], &Token {
            tty: TokenType::CloseBrace,
            lex: "}",
        });
        assert_eq!(&tokens[2], &Token {
            tty: TokenType::OpenBracket,
            lex: "[",
        });
        assert_eq!(&tokens[3], &Token {
            tty: TokenType::CloseBracket,
            lex: "]",
        });
        assert_eq!(&tokens[4], &Token {
            tty: TokenType::Comma,
            lex: ",",
        });
        assert_eq!(&tokens[5], &Token {
            tty: TokenType::Colon,
            lex: ":",
        });


    }

    #[test]
    fn tokenize_whitespace() {
        let source = " \t\n\r\n";
        let tokens = tokenize(source);

        assert_eq!(tokens.len(), 3,
            "number of tokens did not match, tokens = {:?}", tokens
        );
        assert_eq!(&tokens[0], &Token {
            tty: TokenType::Whitespace,
            lex: " \t"
        });
        assert_eq!(&tokens[1], &Token {
            tty: TokenType::Newline,
            lex: "\n",
        });
        assert_eq!(&tokens[2], &Token {
            tty: TokenType::Newline,
            lex: "\r\n",
        });

    }

    #[test]
    fn tokenize_number_whole() {
        let source = "1234567890";
        let tokens = tokenize(source);

        assert_eq!(tokens.len(), 1,
            "number of tokens did not match, tokens = {:?}", tokens
        );
        assert_eq!(&tokens[0], &Token {
            tty: TokenType::Number,
            lex: "1234567890"
        });
    }

    #[test]
    fn tokenize_number_negative() {
        let source = "-3";
        let tokens = tokenize(source);

        assert_eq!(tokens.len(), 1,
            "number of tokens did not match, tokens = {:?}", tokens
        );
        assert_eq!(&tokens[0], &Token {
            tty: TokenType::Number,
            lex: "-3"
        });

    }

    #[test]
    fn tokenize_number_decimal() {
        let source = "3.14";
        let tokens = tokenize(source);

        assert_eq!(tokens.len(), 1,
            "number of tokens did not match, tokens = {:?}", tokens
        );
        assert_eq!(&tokens[0], &Token {
            tty: TokenType::Number,
            lex: "3.14"
        });
    }

    #[test]
    fn tokenize_number_trailing_decimal() {
        let source = "2.";
        let tokens = tokenize(source);

        assert_eq!(tokens.len(), 1,
            "number of tokens did not match, tokens = {:?}", tokens
        );
        assert_eq!(&tokens[0], &Token {
            tty: TokenType::Number,
            lex: "2."
        });
    }

    #[test]
    fn tokenize_number_leading_decimal() {
        let source = ".1";
        let tokens = tokenize(source);

        assert_eq!(tokens.len(), 1,
            "number of tokens did not match, tokens = {:?}", tokens
        );
        assert_eq!(&tokens[0], &Token {
            tty: TokenType::Number,
            lex: ".1"
        })
    }

    #[test]
    fn tokenize_string() {
        let source = "\"hello world 123 - + []\"";
        let tokens = tokenize(source);

        assert_eq!(tokens.len(), 1,
            "number of tokens did not match, tokens = {:?}", tokens
        );
        assert_eq!(&tokens[0], &Token {
            tty: TokenType::String,
            lex: "\"hello world 123 - + []\""
        })
    }

    #[test]
    fn tokenize_string_escaped_quotes() {
        let source = "\"hello world \\\" 123 - + []\"";
        let tokens = tokenize(source);

        assert_eq!(tokens.len(), 1,
            "number of tokens did not match, tokens = {:?}", tokens
        );
        assert_eq!(&tokens[0], &Token {
            tty: TokenType::String,
            lex: "\"hello world \\\" 123 - + []\""
        })
    }

    #[test]
    fn tokenize_invalid_char() {
        let source = "p";
        let tokens = tokenize(source);

        assert_eq!(tokens.len(), 1,
            "number of tokens did not match, tokens = {:?}", tokens
        );
        assert_eq!(&tokens[0], &Token {
            tty: TokenType::InvalidChar,
            lex: "p"
        })
    }
}
