use std::fmt;
use std::marker::PhantomData;

use serde::__private::de::{Content, ContentDeserializer};
use serde::de::{self, IntoDeserializer, MapAccess};
use serde::{Deserialize, de::Visitor};

pub struct TaggedContentVisitor<T> {
    expecting: &'static str,
    value: PhantomData<T>,
}

impl<T> TaggedContentVisitor<T> {
    /// Visitor for the content of an internally tagged enum with the given tag name.
    pub fn new(expecting: &'static str) -> Self {
        TaggedContentVisitor {
            expecting,
            value: PhantomData,
        }
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for TaggedContentVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = (T, Content<'de>);

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.expecting)
    }

    // todo: add support for sequences?
    // fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    // where
    //     S: SeqAccess<'de>,
    // {
    //     let tag = match seq.next_element()? {
    //         Some(tag) => tag,
    //         None => {
    //             return Err(de::Error::missing_field("blerhg".into()));
    //         }
    //     };
    //     let rest = de::value::SeqAccessDeserializer::new(seq);
    //     Ok((tag, Content::deserialize(rest)?))
    // }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut tag = None;
        let mut vec = Vec::<(Content, Content)>::with_capacity(0); // todo
        while let Some(k) = map.next_key()? {
            match k {
                Content::String(_) | Content::Str(_) | Content::Bytes(_) | Content::ByteBuf(_) => {
                    match T::deserialize::<ContentDeserializer<'_, M::Error>>(
                        k.clone().into_deserializer(),
                    ) {
                        // failed to parse a key must be a vlaue
                        Err(_) => {
                            let v = map.next_value()?;
                            vec.push((k, v));
                        }
                        Ok(t) => {
                            if tag.is_some() {
                                // todo: error message
                                return Err(de::Error::duplicate_field("blah".into()));
                            }
                            let v = map.next_value()?;
                            tag = Some(t);
                            vec.push((k, v));
                        }
                    }
                }
                _ => {
                    let v = map.next_value()?;
                    vec.push((k, v));
                }
            };
        }
        match tag {
            None => Err(de::Error::missing_field("tag was not found".into())),
            Some(tag) => Ok((tag, Content::Map(vec))),
        }
    }
}
