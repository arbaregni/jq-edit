use anyhow::Result;
use anyhow::bail;
use itertools::Itertools;

use crate::streaming::Streaming;
use super::tokens::TokenType;
use crate::json::streaming_tokens::Token;

#[derive(Debug)]
pub struct TokenStreamAdaptor<S> {
    tokens: S,
    // remember a peeked value, even if it was None
    peeked: Option<Option<Token>>
}
impl <S> TokenStreamAdaptor<S> where S: Streaming<Item = Token> {
    pub fn from(tokens: S) -> Self {
        Self {
            tokens,
            peeked: None
        }
    }
    fn next_nonspace_token(&mut self) -> Result<Option<Token>> {
        while let Some(tok) = self.tokens.try_next()? {
            if !tok.tty.is_whitespace() {
                return Ok(Some(tok));
            }
        }
        Ok(None)
    }
    pub fn next_token(&mut self) -> Result<Option<Token>> {
        match self.peeked.take() {
            Some(v) => Ok(v),
            None => self.next_nonspace_token()
        }
    }
    pub fn peek(&mut self) -> Result<Option<&Token>> {
        if let None = self.peeked {
            let token = self.next_nonspace_token()?;
            self.peeked = Some(token);
        }
        // SAFETY: if the variant was None, it was set above
        let peeked_item = unsafe {
            self.peeked.as_ref().unwrap_unchecked()
        };
        Ok(peeked_item.as_ref())
    } 
    pub fn consume_if(&mut self, token_type: TokenType) -> Result<Option<Token>> {
        let Some(tok) = self.peek()? else {
            // nothing consumed, nothing to consume
            return Ok(None);
        };
        if tok.tty == token_type {
            self.next_token()
        } else {
            Ok(None)
        }
    }
    pub fn peeked_token_is(&mut self, token_type: TokenType) -> Result<bool> {
        let Some(tok) = self.peek()? else {
            return Ok(false);
        };
        return Ok(tok.tty == token_type) 
    }

    pub fn peek_or_fail(&mut self, token_type: TokenType, message: &str) -> Result<&Token> {
        let Some(tok) = self.peek()? else {
            bail!("unexpected EOF while {message}");
        };
        if tok.tty != token_type {
            bail!("unexpected token {tok:?} while {message}, expected {token_type}");
        }
        Ok(tok)
    }

    pub fn consume_or_fail(&mut self, token_type: TokenType, message: &str) -> Result<Token> {
        let Some(tok) = self.next_token()? else {
            bail!("unexpected EOF while {message}");
        };
        if tok.tty != token_type {
            bail!("unexpected token {tok:?} while {message}, expected {token_type}");
        }
        Ok(tok)
    }
    pub fn consume_one_of_or_fail<const N: usize>(&mut self, token_types: [TokenType; N], message: &str) -> Result<Token> {
        let Some(tok) = self.next_token()? else {
            bail!("unexpected EOF while {message}");
        };
        if token_types.into_iter().all(|tty| tok.tty != tty) {
            bail!("unexpected token {tok:?} while {message}, expected one of: {}", token_types
                  .into_iter()
                  .map(|tty| format!("{tty}"))
                  .join(", ")
           );
        }
        Ok(tok)
    }
}
