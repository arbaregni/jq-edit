use std::borrow::Cow;
use std::path::Path;

use anyhow::Result;

use crate::streaming;

pub mod tokens;

mod streaming_json;
mod streaming_tokens;
mod streaming_json_fragments;
mod token_stream_adaptor;

pub mod tokens_to;

pub fn stream_json_fragments_from_file<P: AsRef<Path>>(path: P) -> Result<impl streaming::Streaming<Item = (JsonPath<'static>, JsonFragment<'static>)>> {
    let br = buffered_reader::File::open(path)?;
    let tokens = streaming_tokens::StreamingTokens::from(br);
    let stream = streaming_json_fragments::StreamingJson::from(tokens);
    Ok(stream)
}
pub fn stream_json_fragments_from<R: std::io::Read + Send + Sync>(reader: R) -> Result<impl streaming::Streaming<Item = (JsonPath<'static>, JsonFragment<'static>)>> {
    let br = buffered_reader::Generic::new(reader, None);
    let tokens = streaming_tokens::StreamingTokens::from(br);
    let stream = streaming_json_fragments::StreamingJson::from(tokens);
    Ok(stream)
}




#[derive(Debug, Clone, PartialEq)]
pub enum JsonFragment<'a> {
    /// Emits when a dictionary begins
    BeginDict,
    /// Emits when a dictionary ends
    EndDict,
    /// Emits when an array begins
    BeginArray,
    /// Emits when an array should be ended
    EndArray,
    /// Emits for a json atom
    Atom(JsonData<'a>),
    /// Emits for invalid data. Can safely ignore
    Invalid(Cow<'a, str>),
}


#[derive(Debug, Clone, PartialEq)]
pub enum JsonData<'a> {
    #[allow(dead_code)]
    Object { entries: Vec<(JsonKey<'a>, JsonData<'a>)> },
    #[allow(dead_code)]
    Array { elems: Vec<JsonData<'a>> },
    Str { value: Cow<'a, str> },
    Boolean { value: bool },
    Number { value: i64 },
    Float { value: f64 },
    Null,
}
impl <'a> JsonData<'a> {
    pub fn empty_array() -> Self {
        Self::Array { elems: vec![] }
    }
    pub fn empty_object() -> Self {
        Self::Object { entries: vec![] }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonKey<'a> {
    inner: Cow<'a, str>,
}
impl <'a> From<&'a str> for JsonKey<'a> {
    fn from(value: &'a str) -> Self {
        Self { inner: Cow::Borrowed(value) }
    }
}impl <'a> From<String> for JsonKey<'a> {
    fn from(value: String) -> Self {
        Self { inner: Cow::Owned(value) }
    }
}impl <'a> From<Cow<'a, str>> for JsonKey<'a> {
    fn from(inner: Cow<'a, str>) -> Self {
        Self { inner }
    }
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
    index_in_stream: usize,
    elems: Vec<JsonPathElement<'a>> // TODO: small vec?
}
impl <'a> JsonPath <'a> {
    // =============================
    //   constructors / builders
    // =============================

    pub fn empty() -> Self {
        Self {
            elems: Vec::new(),
            index_in_stream: 0,
        }
    }    
    #[allow(dead_code)]
    pub fn nth(index_in_stream: usize) -> Self {
        Self {
            elems: Vec::new(),
            index_in_stream
        }
    }
    #[allow(dead_code)]
    pub fn then_index(self, index: usize) -> Self {
        self.with(JsonPathElement::IndexInArray { index })
    }
    #[allow(dead_code)]
    pub fn then_prop<K>(self, key: K) -> Self 
    where K: Into<JsonKey<'a>>
    {
        self.with(JsonPathElement::PropertyInObject { key: key.into() })
    }
    pub fn with(mut self, elem: JsonPathElement<'a>) -> Self {
        self.elems.push(elem);
        self
    }

    // =============================
    //  mutators
    // =============================

    pub fn pop(&mut self) -> Option<JsonPathElement<'a>> {
        self.elems.pop()
    }
    pub fn push(&mut self, elem: JsonPathElement<'a>) {
        self.elems.push(elem);
    }

    /// Warning ! just advances a top level item
    pub fn next_in_stream(&mut self) {
        self.index_in_stream += 1;
    }


}


impl <'a> std::fmt::Display for JsonPath<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.index_in_stream == 0 && self.elems.len() == 0 {
            write!(f, ".")?;
            return Ok(());
        }

        if self.index_in_stream > 0 {
            write!(f, "nth({})", self.index_in_stream)?;
            if self.elems.len() > 0 {
                write!(f, " | ")?;
            }
        }
        for el in &self.elems {
            write!(f, "{el}")?;
        }
        Ok(())
    }
}


