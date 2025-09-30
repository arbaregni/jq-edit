use std::borrow::Cow;

use anyhow::{bail, Result};

use crate::streaming::Streaming;
use crate::json::{
    JsonData, JsonKey, JsonPath, JsonFragment,
    tokens::TokenType,
    streaming_tokens::{Token, tok_to_num, tok_to_str, tok_to_bool},
    token_stream_adaptor::TokenStreamAdaptor,
};


pub type JsonPair<'a> = (JsonPath<'a>, JsonFragment<'a>);

pub struct StreamingJson<'a, T> {
    tokens: TokenStreamAdaptor<T>,
    path: JsonPath<'a>,
    parents: Vec<JsonCover>,
}

#[derive(Debug)]
enum JsonCover {
    Dict { 
        #[allow(dead_code)] start: Token
    },
    Array {
        #[allow(dead_code)] start: Token,
        index: usize
    },
}

impl <'a, T> StreamingJson<'a, T> where T: Streaming<Item = Token> {
    pub fn from(tokens: T) -> Self {
        Self {
            tokens: TokenStreamAdaptor::from(tokens),
            path: JsonPath::empty(),
            parents: Vec::new(),
        }
    }
}


/*

 json = array | string | number | bool

 array = 
    expect "["
    index = 0
    loop:
        path.append(IndexInArray { index })
        yield parse_json()
        path.pop()
        match next_token:
            "," -> index += 1
            "]" -> break

 */

impl <'a, T> Streaming for StreamingJson<'a, T> where T: Streaming<Item = Token> {
    type Item = JsonPair<'a>;

    fn try_next(&mut self) -> Result<Option<JsonPair<'a>>> {
        log::debug!("looking for next token, self.path = {}, self.parents = {:?}", self.path, self.parents);

        // figure out where we are
        let json_path = match self.parents.last() {
            None => { self.path.clone() }
            Some(JsonCover::Array { index, .. }) => {
                if self.tokens.peeked_token_is(TokenType::CloseBracket)? {
                    /* nothing to do */
                    self.path.clone()
                } else {
                    self.path.clone().then_index(*index)
                }
            }
            Some(JsonCover::Dict { .. }) => {
                if self.tokens.peeked_token_is(TokenType::CloseBrace)? {
                    log::debug!("next token is closing dict");
                    self.path.clone()
                } else {
                    // we must look for a key
                    log::debug!("inside object, looking for key value pair");
                    let key_token = self.tokens.consume_or_fail(TokenType::String, "looking for object key")?;
                    log::debug!("found key:object");
                    self.tokens.consume_or_fail(TokenType::Colon, "looking for object key")?;
                    let key_value = key_token.lex; // enquote::unquote(&key_token.lex)?;
                    let prop = JsonKey::from(key_value); 
                    self.path.clone().then_prop(prop)
                }
            }
        };
        log::debug!("path to use: {json_path:?}");

        let Some(tok) = self.tokens.next_token()? else {
            // found EOF - is it Okay to end here?
            match self.parents.last() {
                None => {
                    // okay to end here
                    return Ok(None);
                }
                Some(JsonCover::Array { start, .. }) => {
                    bail!("unexpected EOF while working on array starting at {start}");
                }
                Some(JsonCover::Dict { start, .. }) => {
                    bail!("unexpected EOF while working on dictionary starting from {start}");
                }
            }
        };
        log::debug!("next token: {tok:?}");

        // figure out what we are
        let json_fragment = match &tok.tty {
            TokenType::String => JsonFragment::Atom(tok_to_str(&tok)?),
            TokenType::Number => JsonFragment::Atom(tok_to_num(&tok)?),
            TokenType::Boolean => JsonFragment::Atom(tok_to_bool(&tok)?),
            TokenType::Null => JsonFragment::Atom(JsonData::Null),
            TokenType::OpenBrace => {
                self.parents.push(JsonCover::Dict { start: tok });
                // we need to modify the path here?
                self.path = json_path.clone();
                log::debug!("we are saving our path after entering a new obejct, self.path = {json_path:?}");
                JsonFragment::BeginDict
            }
            TokenType::OpenBracket => {
                self.parents.push(JsonCover::Array { start: tok, index: 0 });
                // we need to modify the path here?
                self.path = json_path.clone();
                log::debug!("we are saving our path after entering a new obejct, self.path = {json_path:?}");
                JsonFragment::BeginArray
            }
            TokenType::CloseBrace => {
                match self.parents.last() {
                    None => bail!("invalid - closing {} outside of dictionary context", tok.tty),
                    Some(JsonCover::Dict { .. }) => (), // no issue
                    Some(JsonCover::Array { start, .. }) => bail!("invalid - closing array starting at {:?} with {}", start, tok.tty)
                };
                self.parents.pop();
                self.path.pop();
                log::debug!("found close brace, new path = {json_path:?}");
                JsonFragment::EndDict
            }
            TokenType::CloseBracket => {
                match self.parents.last() {
                    None => bail!("invalid - closing {} outside of array context", tok.tty),
                    Some(JsonCover::Dict { start, .. }) => bail!("invalid - closing dictionary starting at {:?} with {}", start, tok.tty),
                    Some(JsonCover::Array { .. }) => (), // no issue
                };
                self.parents.pop();
                self.path.pop();
                log::debug!("found close bracket, new path = {json_path:?}");
                JsonFragment::EndArray
            }
            TokenType::InvalidChar => JsonFragment::Invalid(Cow::Owned(tok.lex)),
            _ => bail!("unexpected token: {tok}"),
        };

        log::debug!("yielding: {json_path:?}, {json_fragment:?}");

        // stream counter
        match self.parents.last_mut() {
            None => {
                self.path.next_in_stream();
            }
            Some(_) if json_fragment == JsonFragment::BeginDict || json_fragment == JsonFragment::BeginArray => { /* nothing to do */ }
            Some(JsonCover::Dict { .. }) => {
                let found_comma = self.tokens.consume_if(TokenType::Comma)?.is_some();
                if !found_comma {
                    self.tokens.peek_or_fail(TokenType::CloseBrace, "after dictionary item when no comma was found")?;
                }
            }
            Some(JsonCover::Array { index, .. }) => {
                let found_comma = self.tokens.consume_if(TokenType::Comma)?.is_some();
                if found_comma {
                    log::debug!("inside array, advancing to next item");
                    *index += 1;
                } else {
                    self.tokens.peek_or_fail(TokenType::CloseBracket, "after array item when no comma was found")?;
                }
            }
        }

        let json_pair = (json_path, json_fragment);
        Ok(Some(json_pair))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn make_stream(input: &str) -> impl Streaming<Item = JsonPair<'static>> + use<'_> {
        let br = buffered_reader::Memory::new(input.as_bytes());
        let tokens = crate::json::streaming_tokens::StreamingTokens::from(br);
        StreamingJson::from(tokens)
    }

    fn json_int(value: i64) -> JsonFragment<'static> {
        JsonFragment::Atom(JsonData::Number { value })
    }
    fn json_float(value: f64) -> JsonFragment<'static> {
        JsonFragment::Atom(JsonData::Float { value })
    }
    fn json_bool(value: bool) -> JsonFragment<'static> {
        JsonFragment::Atom(JsonData::Boolean { value })
    }
    fn json_str<'a>(value: &'a str) -> JsonFragment<'a> {
        JsonFragment::Atom(JsonData::Str { value: Cow::from(value) })
    }
    fn json_null() -> JsonFragment<'static> {
        JsonFragment::Atom(JsonData::Null)
    }

    macro_rules! assert_stream_eqs {
        ( $stream:expr, [ $( $expected:expr ),* $(,)? ]) => {
            $(
                assert_eq!($stream.try_next().expect("to succeed"), $expected);
            )*
        };
    }

    macro_rules! atomic_test_cases {
        ( $( $test_case_name:ident $test_input:expr => $expected:expr ; )* ) => {
            $(
                #[test]
                fn $test_case_name() {
                    let mut s = make_stream($test_input);
                    assert_stream_eqs!(s, [
                        Some((JsonPath::nth(0), $expected)),
                        None,
                    ]);
                }
            )*
        };
    }

    atomic_test_cases! {
        test_whole_number "123" => json_int(123) ;
        test_bool_true "true" => json_bool(true) ;
        test_bool_false "false" => json_bool(false) ;
        test_null "null" => json_null();

        test_float_simple "12.3" => json_float(12.3) ;
        test_float_leading_period ".3" => json_float(0.3) ;
        test_float_following_period "3." => json_float(3.0) ;

        test_scientific "1e2" => json_int(100) ;
        test_float_scientific "1.0e2" => json_float(100.0) ;
        test_float_scientific_negative_exp "1.0e-2" => json_float(0.01) ;
    }

    #[test]
    fn test_atoms_stream() {
        let mut s = make_stream("12 true \"hello\"");
        assert_stream_eqs!(s, [
            Some((JsonPath::nth(0), json_int(12))),
            Some((JsonPath::nth(1), json_bool(true))),
            Some((JsonPath::nth(2), json_str("hello"))),
            None,
        ]);
    }

    #[test]
    fn test_array_simple() {
        let mut s = make_stream("[12, true, \"hello\"]");
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0), json_int(12))),
            Some((JsonPath::empty().then_index(1), json_bool(true))),
            Some((JsonPath::empty().then_index(2), json_str("hello"))),
            Some((JsonPath::empty(), JsonFragment::EndArray)),
            None,
        ]);
    }

    #[test]
    fn test_array_should_error() {
        let mut s = make_stream("[12, ");
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0), json_int(12))),
        ]);
        s.try_next().expect_err("should fail");
    }
    #[test]
    fn test_array_nested_simple() {
        let mut s = make_stream("[[1, 2]]");
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0).then_index(0), json_int(1))),
            Some((JsonPath::empty().then_index(0).then_index(1), json_int(2))),
            Some((JsonPath::empty().then_index(0), JsonFragment::EndArray)),
            Some((JsonPath::empty(), JsonFragment::EndArray)),
            None,
        ]);
    }

    #[test]
    fn test_array_empty() {
        let mut s = make_stream("[]");
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginArray)),
            Some((JsonPath::empty(), JsonFragment::EndArray)),
            None,
        ]);
    }

    #[test]
    fn test_dict_simple() {
        let mut s = make_stream(r#"{"user.name": "dogwart", "user.id": 123, "user.isPaying": true }"#);
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_prop("user.name"), json_str("dogwart"))),
            Some((JsonPath::empty().then_prop("user.id"), json_int(123))),
            Some((JsonPath::empty().then_prop("user.isPaying"), json_bool(true))),
            Some((JsonPath::empty(), JsonFragment::EndDict)),
            None,
        ]);
    }

    #[test]
    fn test_dict_empty() {
        let mut s = make_stream("{}");
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginDict)),
            Some((JsonPath::empty(), JsonFragment::EndDict)),
            None,
        ]);
    }



    #[test]
    fn test_dict_simple_nexted() {
        let mut s = make_stream(r#"{"network": { "ip": "169.254.169.254" }, "tags": { "name": "jump host" } }"#);
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_prop("network"), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_prop("network").then_prop("ip"), json_str("169.254.169.254"))),
            Some((JsonPath::empty().then_prop("network"), JsonFragment::EndDict)),
            Some((JsonPath::empty().then_prop("tags"), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_prop("tags").then_prop("name"), json_str("jump host"))),
            Some((JsonPath::empty().then_prop("tags"), JsonFragment::EndDict)),
            Some((JsonPath::empty(), JsonFragment::EndDict)),
            None,
        ]);
    }

    #[test]
    fn test_nested_arrays() {
        let mut s = make_stream("[[1, 2], [3, 4]]");
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0).then_index(0), json_int(1))),
            Some((JsonPath::empty().then_index(0).then_index(1), json_int(2))),
            Some((JsonPath::empty().then_index(0), JsonFragment::EndArray)),
            Some((JsonPath::empty().then_index(1), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(1).then_index(0), json_int(3))),
            Some((JsonPath::empty().then_index(1).then_index(1), json_int(4))),
            Some((JsonPath::empty().then_index(1), JsonFragment::EndArray)),
            Some((JsonPath::empty(), JsonFragment::EndArray)),
            None,
        ]);
    }

    #[test]
    fn test_array_of_objects() {
        let mut s = make_stream(r#"[{"id": 1}, {"id": 2}, {"id": 3}]"#);
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_index(0).then_prop("id"), json_int(1))),
            Some((JsonPath::empty().then_index(0), JsonFragment::EndDict)),
            Some((JsonPath::empty().then_index(1), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_index(1).then_prop("id"), json_int(2))),
            Some((JsonPath::empty().then_index(1), JsonFragment::EndDict)),
            Some((JsonPath::empty().then_index(2), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_index(2).then_prop("id"), json_int(3))),
            Some((JsonPath::empty().then_index(2), JsonFragment::EndDict)),
            Some((JsonPath::empty(), JsonFragment::EndArray)),
            None,
        ]);
    }
    #[test]
    fn test_object_of_arrays() {
        let mut s = make_stream(r#"{"colors": ["blue", "red", "purple"]}"#);
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_prop("colors"), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_prop("colors").then_index(0), json_str("blue"))),
            Some((JsonPath::empty().then_prop("colors").then_index(1), json_str("red"))),
            Some((JsonPath::empty().then_prop("colors").then_index(2), json_str("purple"))),
            Some((JsonPath::empty().then_prop("colors"), JsonFragment::EndArray)),
            Some((JsonPath::empty(), JsonFragment::EndDict)),
            None,
        ]);
    }

    #[test]
    fn test_array_nested_binary_tree() {
        let mut s = make_stream("[[[0, 1], [2, 3]], [[4, 5], [6, 7]]]");
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0).then_index(0), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0).then_index(0).then_index(0), json_int(0))),
            Some((JsonPath::empty().then_index(0).then_index(0).then_index(1), json_int(1))),
            Some((JsonPath::empty().then_index(0).then_index(0), JsonFragment::EndArray)),
            Some((JsonPath::empty().then_index(0).then_index(1), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0).then_index(1).then_index(0), json_int(2))),
            Some((JsonPath::empty().then_index(0).then_index(1).then_index(1), json_int(3))),
            Some((JsonPath::empty().then_index(0).then_index(1), JsonFragment::EndArray)),
            Some((JsonPath::empty().then_index(0), JsonFragment::EndArray)),
            Some((JsonPath::empty().then_index(1), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(1).then_index(0), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(1).then_index(0).then_index(0), json_int(4))),
            Some((JsonPath::empty().then_index(1).then_index(0).then_index(1), json_int(5))),
            Some((JsonPath::empty().then_index(1).then_index(0), JsonFragment::EndArray)),
            Some((JsonPath::empty().then_index(1).then_index(1), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(1).then_index(1).then_index(0), json_int(6))),
            Some((JsonPath::empty().then_index(1).then_index(1).then_index(1), json_int(7))),
            Some((JsonPath::empty().then_index(1).then_index(1), JsonFragment::EndArray)),
            Some((JsonPath::empty().then_index(1), JsonFragment::EndArray)),
            Some((JsonPath::empty(), JsonFragment::EndArray)),
            None,
        ]);
    }

    #[test]
    fn test_array_with_trailing_comma() {
        let mut s = make_stream("[1, 2,]");
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_index(0), json_int(1))),
            Some((JsonPath::empty().then_index(1), json_int(2))),
            Some((JsonPath::empty(), JsonFragment::EndArray)),
            None,
        ]);
    }

    #[test]
    fn test_json_literals() {
        let mut s = make_stream("true false null");
        assert_stream_eqs!(s, [
            Some((JsonPath::nth(0), json_bool(true))),
            Some((JsonPath::nth(1), json_bool(false))),
            Some((JsonPath::nth(2), json_null())),
            None,
        ]);
    }

    #[test]
    fn test_unicode_characters() {
        let mut s = make_stream(r#"{"emoji": "😀", "language": "日本語"}"#);
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_prop("emoji"), json_str("😀"))),
            Some((JsonPath::empty().then_prop("language"), json_str("日本語"))),
            Some((JsonPath::empty(), JsonFragment::EndDict)),
            None,
        ]);
    }

    #[test]
    fn test_nested_arrays_and_objects() {
        let mut s = make_stream(r#"{"users": [{"name": "alice"}, {"name": "bob"}]}"#);
        assert_stream_eqs!(s, [
            Some((JsonPath::empty(), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_prop("users"), JsonFragment::BeginArray)),
            Some((JsonPath::empty().then_prop("users").then_index(0), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_prop("users").then_index(0).then_prop("name"), json_str("alice"))),
            Some((JsonPath::empty().then_prop("users").then_index(0), JsonFragment::EndDict)),
            Some((JsonPath::empty().then_prop("users").then_index(1), JsonFragment::BeginDict)),
            Some((JsonPath::empty().then_prop("users").then_index(1).then_prop("name"), json_str("bob"))),
            Some((JsonPath::empty().then_prop("users").then_index(1), JsonFragment::EndDict)),
            Some((JsonPath::empty().then_prop("users"), JsonFragment::EndArray)),
            Some((JsonPath::empty(), JsonFragment::EndDict)),
            None,
        ]);
    }



    #[test]
    fn test_large_json_object() {
        // Create a large object with many keys: {"key0": 0, "key1": 1, ..., "key99": 99}
        let mut json_string = String::from("{");
        for i in 0..100 {
            if i > 0 {
                json_string.push_str(", ");
            }
            json_string.push_str(&format!(r#""key{}": {}"#, i, i));
        }
        json_string.push('}');

        let mut s = make_stream(&json_string);
        assert_eq!(s.try_next().expect("to succeed"), Some((JsonPath::empty(), JsonFragment::BeginDict)));
        for i in 0..100 {

            let key = format!("key{}", i);
            assert_eq!(
                s.try_next().unwrap(),
                Some((JsonPath::empty().then_prop(key), json_int(i)))
            );
        }
        assert_eq!(s.try_next().expect("to succeed"), Some((JsonPath::empty(), JsonFragment::EndDict)));
        assert_eq!(s.try_next().unwrap(), None);
    }

    #[test]
    #[ignore]
    fn test_invalid_float_format_should_error() {
        let mut s = make_stream("1..0");
        s.try_next().expect_err("Invalid float format should cause error");
    }
}
