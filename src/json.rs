use std::borrow::Cow;

mod parsing;
pub mod tokens;
pub mod streaming;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonData<'a> {
    Object { entries: Vec<(JsonKey<'a>, JsonData<'a>)> },
    Array { elems: Vec<JsonData<'a>> },
    Str { lex: Cow<'a, str> },
    Boolean { value: bool },
    Number { value: i64 }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonKey<'a> {
    inner: Cow<'a, str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonPathElement<'a> {
    PropertyInObject { key: JsonKey<'a> },
    IndexInArray { index: usize },
}
impl <'a> std::fmt::Display for JsonPathElement<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonPathElement::PropertyInObject { key } => if key.inner.chars().all(|ch| ch.is_ascii_alphanumeric()) {
                write!(f, ".{}", key.inner.as_ref())
            } else {
                write!(f, ".{:?}", key.inner.as_ref())
            },
            JsonPathElement::IndexInArray { index } => write!(f, "[{index}]")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPath<'a> {
    elems: Vec<JsonPathElement<'a>>
}

impl <'a> std::fmt::Display for JsonPath<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for el in &self.elems {
            write!(f, "{el}")?;
        }
        Ok(())
    }
}


