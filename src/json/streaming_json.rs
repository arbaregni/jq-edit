use anyhow::{bail, Result};

use std::borrow::Cow;

use crate::streaming::Streaming;
use crate::json::{
    JsonData, JsonKey, JsonPath, JsonPathElement,
    tokens::TokenType,
    streaming_tokens::{Token, tok_to_num, tok_to_str, tok_to_bool},
};

use super::token_stream_adaptor::TokenStreamAdaptor;

pub type JsonPair<'a> = (JsonPath<'a>, JsonData<'a>);

pub struct StreamingJson<'a, T> {
    tokens: TokenStreamAdaptor<T>,
    path: JsonPath<'a>,
    parents: Vec<JsonBegin>,
}

enum JsonBegin {
    Object { 
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
        // pre-matching
        let json_pair = loop {
            match self.parents.last() {
                None => {
                    // nothing to do
                    log::debug!("inside streaming mode, path = {}", self.path);
                }
                Some(JsonBegin::Array { index, .. }) => {
                    self.path.push(JsonPathElement::IndexInArray { index: *index });
                    log::debug!("inside array, modifying path {}", self.path);
                }
                Some(JsonBegin::Object { .. }) => {
                    // we must look for a key
                    log::debug!("inside object, modifying path");
                    let key_token = self.tokens.consume_or_fail(TokenType::String, "looking for object key")?;
                    self.tokens.consume_or_fail(TokenType::Colon, "looking for object key")?;

                    let key = JsonKey::from(key_token.lex.clone()); // TODO: quote it properly
                    self.path.push(JsonPathElement::PropertyInObject { key });
                    log::debug!("inside array,path = {}", self.path);
                }
            }

            let Some(tok) = self.tokens.next_token()? else {
                // found EOF - is it Okay to end here?
                match self.parents.last() {
                    None => {
                        // okay to end here
                        return Ok(None);
                    }
                    Some(JsonBegin::Array { .. }) => {
                        bail!("unexpected EOF while working on array");
                    }
                    Some(JsonBegin::Object { .. }) => {
                        bail!("unexpected EOF while working on object");
                    }
                }
            };
            log::debug!("eating token: {tok:?}");

            let json_data = match &tok.tty {
                TokenType::String => tok_to_str(&tok)?,
                TokenType::Number => tok_to_num(&tok)?,
                TokenType::Boolean => tok_to_bool(&tok)?,
                TokenType::Null => JsonData::Null,
                TokenType::OpenBracket => {
                    log::debug!("enter array");
                    if let Some(_tok) = self.tokens.consume_if(TokenType::CloseBracket)? {
                        log::debug!("found close bracket immeditely afterwords, emit empty array");
                        JsonData::empty_array()
                    } else {
                        self.parents.push(JsonBegin::Array { start: tok, index: 0 });
                        continue; // keep searching until we get to a real json object
                    }
                },
                TokenType::OpenBrace => {
                    log::debug!("enter brace");
                    if let Some(_tok) = self.tokens.consume_if(TokenType::CloseBrace)? {
                        log::debug!("found close bracket immeditely afterwords, emit empty array");
                        JsonData::empty_object()
                    } else {
                        self.parents.push(JsonBegin::Object { start: tok });
                        continue; // keep searching until we get to a real json object
                    }
                },
                _ => bail!("Unexpected token: {tok:?}. Expected '[', '{{', string, number, or boolean"),
            };
            let json_path = self.path.clone();
            break (json_path, json_data);
        };


        log::debug!("json pair found = {json_pair:?}, restoring path: path = {}", self.path);



        // post-matching
        loop { 
            self.path.pop();
            match self.parents.last_mut() {
                None => {
                    log::debug!("moving to next in stream");
                    self.path.next_in_stream();
                    break;
                }
                Some(JsonBegin::Array { index, .. }) => {
                    // we must needs a comma or to leave the array
                    let delim = self.tokens.consume_one_of_or_fail([TokenType::CloseBracket, TokenType::Comma], "after array element")?;
                    match &delim.tty {
                        TokenType::CloseBracket => {
                            log::debug!("exit array");
                            self.parents.pop();
                        }
                        TokenType::Comma => {
                            log::debug!("moving to next in array");
                            *index += 1;
                            break;
                        }
                        _ => bail!("invalid tokentype: {delim:?} after array element (this is likely a bug, it should have better message)")
                    }
                }
                Some(JsonBegin::Object { .. }) => {
                    let delim = self.tokens.consume_one_of_or_fail([TokenType::CloseBrace, TokenType::Comma], "after object key")?;
                    match &delim.tty {
                        TokenType::CloseBrace => {
                            log::debug!("exit obejct");
                            self.parents.pop();
                        }
                        TokenType::Comma => {
                            log::debug!("moving to next entry in object");
                            break;
                        }
                        _ => bail!("invalid tokentype: {delim:?} after object key (this is likely a bug, it should have better message)")
                    }
                }
            }
        }

        // actually emit it here. It's a bit weird that we do it here (there could be some
        // errors in the above which could prevent us from emitting real data).
        // Being lazy in keeping it like this.
        return Ok(Some(json_pair))
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

    fn json_int(value: i64) -> JsonData<'static> {
        JsonData::Number { value }
    }
    fn json_float(value: f64) -> JsonData<'static> {
        JsonData::Float { value }
    }

    fn json_bool(value: bool) -> JsonData<'static> {
        JsonData::Boolean { value }
    }
    fn json_str(value: &str) -> JsonData {
        JsonData::Str { value: Cow::from(value) }
    }

    macro_rules! atomic_test_cases {
        ( $( $test_case_name:ident $test_input:expr => $expected:expr ; )* ) => {

            $(
                #[test]
                fn $test_case_name() {
                    let mut s = make_stream($test_input);
                    assert_eq!(s.try_next().expect("to succeed"), Some((JsonPath::nth(0), $expected)));
                    assert_eq!(s.try_next().expect("to succeed"), None);
                }
            )*

        };
    }

    atomic_test_cases! {
        test_whole_number "123" => json_int(123) ;
        test_bool_true "true" => json_bool(true) ;
        test_bool_false "false" => json_bool(false) ;
        test_null "null" => JsonData::Null ;

        test_empty_array "[]" => JsonData::empty_array() ;
        test_empty_object "{}" => JsonData::empty_object() ;

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
        assert_eq!(s.try_next().expect("to succeed"), Some((JsonPath::nth(0), json_int(12))));
        assert_eq!(s.try_next().expect("to succeed"), Some((JsonPath::nth(1), json_bool(true)))); 
        assert_eq!(s.try_next().expect("to succeed"), Some((JsonPath::nth(2), json_str("hello"))));
        assert_eq!(s.try_next().expect("to succeed"), None);
    }
    #[test]
    fn test_array_simple() {
        let mut s = make_stream("[12, true, \"hello\"]");
        assert_eq!(s.try_next().expect("to succeed"), Some((JsonPath::empty().then_index(0), json_int(12))));
        assert_eq!(s.try_next().expect("to succeed"), Some((JsonPath::empty().then_index(1), json_bool(true)))); 
        assert_eq!(s.try_next().expect("to succeed"), Some((JsonPath::empty().then_index(2), json_str("hello"))));
        assert_eq!(s.try_next().expect("to succeed"), None);
    }

    #[test]
    fn test_array_should_error() {
        let mut s = make_stream("[12, ");
        assert_eq!(s.try_next().expect("to succeed"), Some((JsonPath::empty().then_index(0), json_int(12))));
        s.try_next().expect_err("should fail");
    }

    #[test]
    fn test_array_nested_left() {
        let mut s = make_stream("[ [ [1, 2], 3, 4], 5, 6]");
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(0).then_index(0).then_index(0),
                    json_int(1)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(0).then_index(0).then_index(1),
                    json_int(2)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(0).then_index(1),
                    json_int(3)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(0).then_index(2),
                    json_int(4)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(1),
                    json_int(5)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(2),
                    json_int(6)
        )));
        assert_eq!(s.try_next().expect("to succeed"), None);
    }

    #[test]
    fn test_array_nested_binary_tree() {
        let mut s = make_stream("[[[0, 1], [2, 3]], [[4, 5], [6, 7]]]");
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(0).then_index(0).then_index(0),
                    json_int(0)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(0).then_index(0).then_index(1),
                    json_int(1)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(0).then_index(1).then_index(0),
                    json_int(2)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(0).then_index(1).then_index(1),
                    json_int(3)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(1).then_index(0).then_index(0),
                    json_int(4)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(1).then_index(0).then_index(1),
                    json_int(5)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(1).then_index(1).then_index(0),
                    json_int(6)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_index(1).then_index(1).then_index(1),
                    json_int(7)
        )));
        assert_eq!(s.try_next().expect("to succeed"), None);
    }



    #[test]
    fn test_object_simple() {
        let mut s = make_stream(r#"{"foo": 12, "bar": true, "baz": "hello"}"#);
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_prop("foo"), 
                    json_int(12)
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_prop("bar"),
                    json_bool(true)
        ))); 
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_prop("baz"),
                    json_str("hello")
        )));
        assert_eq!(s.try_next().expect("to succeed"), None);
    }


    #[test]
    fn test_object_nested() {
        let mut s = make_stream(r#"{"foo": { "bar": { "baz": "hello0", "baz1": "hello1" }, "bar2": "hello2" }, "foo2": "hello3" }"#);
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_prop("foo").then_prop("bar").then_prop("baz"), 
                    json_str("hello0"),
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_prop("foo").then_prop("bar").then_prop("baz1"), 
                    json_str("hello1"),
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_prop("foo").then_prop("bar2"),
                    json_str("hello2"),
        )));
        assert_eq!(s.try_next().expect("to succeed"), Some((
                    JsonPath::empty().then_prop("foo2"),
                    json_str("hello3"),
        )));
        assert_eq!(s.try_next().expect("to succeed"), None);
    }


    #[test]
    fn test_multiple_concatenated_objects() {
        let mut s = make_stream(r#"{"user":"alice"}{"user":"bob"}{"user":"carol"}"#);
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::nth(0).then_prop("user"), json_str("alice")
        )));
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::nth(1).then_prop("user"), json_str("bob")
        )));
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::nth(2).then_prop("user"), json_str("carol")
        )));
        assert_eq!(s.try_next().unwrap(), None);
    }

    #[test]
    fn test_objects_with_whitespace() {
        let mut s = make_stream(r#"{"a":1} 
        {"b":2}
        {"c":3}"#);
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::nth(0).then_prop("a"), json_int(1)
        )));
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::nth(1).then_prop("b"), json_int(2)
        )));
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::nth(2).then_prop("c"), json_int(3)
        )));
        assert_eq!(s.try_next().unwrap(), None);
    }

    #[test]
    fn test_empty_input_returns_none() {
        let mut s = make_stream("");
        assert_eq!(s.try_next().unwrap(), None);
    }

    #[test]
    fn test_incomplete_json_raises_error() {
        // TODO - this could return a complete token, but instead it doesn't. oh well.
        let mut s = make_stream(r#"{"user": "bob""#); // Missing closing brace
        assert!(s.try_next().is_err(), "Expected error on incomplete JSON");
    }

    #[test]
    fn test_escaped_characters_in_string() {
        let mut s = make_stream(r#"{"quote": "He said, \"hello\"."}"#);
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::empty().then_prop("quote"),
            json_str(r#"He said, "hello"."#),
        )));
        assert_eq!(s.try_next().unwrap(), None);
    }

    #[test]
    fn test_json_literals() {
        let mut s = make_stream("true false null");
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::nth(0), json_bool(true))));
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::nth(1), json_bool(false))));
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::nth(2), JsonData::Null)));
        assert_eq!(s.try_next().unwrap(), None);
    }

    #[test]
    fn test_unicode_characters() {
        let mut s = make_stream(r#"{"emoji": "😀", "language": "日本語"}"#);
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::empty().then_prop("emoji"),
            json_str("😀")
        )));
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::empty().then_prop("language"),
            json_str("日本語")
        )));
        assert_eq!(s.try_next().unwrap(), None);
    }

    #[test]
    fn test_nested_arrays_and_objects() {
        let mut s = make_stream(r#"{"users": [{"name": "alice"}, {"name": "bob"}]}"#);
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::empty().then_prop("users").then_index(0).then_prop("name"),
            json_str("alice")
        )));
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::empty().then_prop("users").then_index(1).then_prop("name"),
            json_str("bob")
            )));
            assert_eq!(s.try_next().unwrap(), None);
        }

    #[test]
    fn test_array_of_objects() {
        let mut s = make_stream(r#"[{"id": 1}, {"id": 2}, {"id": 3}]"#);
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::empty().then_index(0).then_prop("id"),
            json_int(1)
        )));
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::empty().then_index(1).then_prop("id"),
            json_int(2)
        )));
        assert_eq!(s.try_next().unwrap(), Some((
            JsonPath::empty().then_index(2).then_prop("id"),
            json_int(3)
        )));
        assert_eq!(s.try_next().unwrap(), None);
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
        for i in 0..100 {
            let key = format!("key{}", i);
            assert_eq!(s.try_next().unwrap(), Some((
                JsonPath::empty().then_prop(key),
                json_int(i),
            )));
        }
        assert_eq!(s.try_next().unwrap(), None);
    }

    #[test]
    fn test_nested_arrays() {
        let mut s = make_stream("[[1, 2], [3, 4]]");
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::empty().then_index(0).then_index(0), json_int(1))));
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::empty().then_index(0).then_index(1), json_int(2))));
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::empty().then_index(1).then_index(0), json_int(3))));
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::empty().then_index(1).then_index(1), json_int(4))));
        assert_eq!(s.try_next().unwrap(), None);
    }

    #[test]
    fn test_mixed_type_array() {
        let mut s = make_stream("[42, true, \"hi\"]");
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::empty().then_index(0), json_int(42))));
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::empty().then_index(1), json_bool(true))));
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::empty().then_index(2), json_str("hi"))));
        assert_eq!(s.try_next().unwrap(), None);
    }

    #[test]
    fn test_array_with_trailing_comma_should_error() {
        let mut s = make_stream("[1, 2,]");
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::empty().then_index(0), json_int(1))));
        assert_eq!(s.try_next().unwrap(), Some((JsonPath::empty().then_index(1), json_int(2))));
        assert!(s.try_next().is_err(), "Trailing comma should cause error");
    }

    #[test]
    fn test_invalid_float_format_should_error() {
        let mut s = make_stream("[1..0]");
        assert!(s.try_next().is_err(), "Invalid float format should cause error");
    }
}

