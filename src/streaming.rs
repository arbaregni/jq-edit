use anyhow::Result;

pub trait Streaming {
    type Item;

    fn try_next(&mut self) -> Result<Option<Self::Item>>;
}
