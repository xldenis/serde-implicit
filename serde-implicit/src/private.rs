use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, IntoDeserializer, MapAccess, Unexpected};
use serde::forward_to_deserialize_any;
use serde::{Deserialize, de::Visitor};

pub use crate::content::{Content, ContentDeserializer, ContentRefDeserializer};

pub struct TaggedContentVisitor<T> {
    expecting: &'static str,
    fallthrough: Option<T>,
}

impl<T> TaggedContentVisitor<T> {
    /// Visitor for the content of an internally tagged enum with the given tag name.
    pub fn new(expecting: &'static str, fallthrough: Option<T>) -> Self {
        TaggedContentVisitor {
            expecting,
            fallthrough,
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

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.fallthrough {
            Some(default) => Ok((default, Content::String(v.into()))),
            None => Err(de::Error::invalid_type(Unexpected::Str(v), &self.expecting)),
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.fallthrough {
            Some(default) => Ok((default, Content::U64(v))),
            None => Err(de::Error::invalid_type(
                Unexpected::Unsigned(v),
                &self.expecting,
            )),
        }
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.fallthrough {
            Some(default) => Ok((default, Content::I64(v))),
            None => Err(de::Error::invalid_type(
                Unexpected::Signed(v),
                &self.expecting,
            )),
        }
    }

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
        match (tag, self.fallthrough) {
            (None, None) => Err(de::Error::missing_field("tag was not found".into())),
            (None, Some(default)) => Ok((default, Content::Map(vec))),
            (Some(tag), _) => Ok((tag, Content::Map(vec))),
        }
    }
}

/// Deserialize a missing field. If the field is `Option<T>` this returns
/// `None`. For all other types, returns a "missing field" error. This
/// replicates the same mechanism used by serde's own derive macro.
pub fn missing_field<'de, V, E>(field: &'static str) -> Result<V, E>
where
    V: Deserialize<'de>,
    E: de::Error,
{
    struct MissingFieldDeserializer<E>(&'static str, PhantomData<E>);

    impl<'de, E> serde::Deserializer<'de> for MissingFieldDeserializer<E>
    where
        E: de::Error,
    {
        type Error = E;

        fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, E>
        where
            V: Visitor<'de>,
        {
            Err(de::Error::missing_field(self.0))
        }

        fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, E>
        where
            V: Visitor<'de>,
        {
            visitor.visit_none()
        }

        forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }

    let deserializer = MissingFieldDeserializer(field, PhantomData);
    Deserialize::deserialize(deserializer)
}

pub fn extract_at_index<'de, E: serde::de::Error>(
    c: Content<'de>,
    index: usize,
) -> ::std::result::Result<(Content<'de>, Option<Content<'de>>), E> {
    match c {
        Content::Seq(mut s) => {
            if s.len() == 0 {
                Err(E::missing_field("missing tag: sequence is empty"))
            } else if index >= s.len() {
                Err(E::missing_field("tag index out of bounds"))
            } else {
                Ok((s.remove(index), Some(Content::Seq(s))))
            }
        }
        c => {
            if index == 0 {
                Ok((c, None))
            } else {
                Err(E::missing_field("tag index out of bounds for non-sequence"))
            }
        }
    }
}
