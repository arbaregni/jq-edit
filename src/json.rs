use anyhow::{
    Result,
    anyhow
};

use std::borrow::Cow;

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

pub fn loads<'a>(source: &'a str) -> Result<JsonData<'a>> {
    Err(anyhow!("not implemented"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int() {
        let source = "34";
        let expected = JsonData(JsonDataInner {
            ty: JsonDataType::Number,
            lex: Cow::from("34")
        });

        let actual = loads(source).expect("this should parse");

        assert_eq!(actual, expected);
    }
}

