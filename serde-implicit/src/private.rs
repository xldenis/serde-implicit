use std::fmt;

use serde::__private::de::{Content, ContentDeserializer};
use serde::de::{self, IntoDeserializer, MapAccess, Unexpected};
use serde::{Deserialize, de::Visitor};

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

pub fn pop_front<'de, E: serde::de::Error>(
    c: Content<'de>,
) -> serde::__private::Result<(Content<'de>, Option<Content<'de>>), E> {
    match c {
        Content::Seq(mut s) => {
            if s.len() == 0 {
                serde::__private::de::missing_field("missing tag")
            } else {
                Ok((s.remove(0), Some(Content::Seq(s))))
            }
        }
        c => Ok((c, None)),
    }
}

pub fn unexpected<'a>(c: &'a Content<'_>) -> serde::de::Unexpected<'a> {
    use serde::de::Unexpected;
    match *c {
        Content::Bool(b) => Unexpected::Bool(b),
        Content::U8(n) => Unexpected::Unsigned(n as u64),
        Content::U16(n) => Unexpected::Unsigned(n as u64),
        Content::U32(n) => Unexpected::Unsigned(n as u64),
        Content::U64(n) => Unexpected::Unsigned(n),
        Content::I8(n) => Unexpected::Signed(n as i64),
        Content::I16(n) => Unexpected::Signed(n as i64),
        Content::I32(n) => Unexpected::Signed(n as i64),
        Content::I64(n) => Unexpected::Signed(n),
        Content::F32(f) => Unexpected::Float(f as f64),
        Content::F64(f) => Unexpected::Float(f),
        Content::Char(c) => Unexpected::Char(c),
        Content::String(ref s) => Unexpected::Str(s),
        Content::Str(s) => Unexpected::Str(s),
        Content::ByteBuf(ref b) => Unexpected::Bytes(b),
        Content::Bytes(b) => Unexpected::Bytes(b),
        Content::None | Content::Some(_) => Unexpected::Option,
        Content::Unit => Unexpected::Unit,
        Content::Newtype(_) => Unexpected::NewtypeStruct,
        Content::Seq(_) => Unexpected::Seq,
        Content::Map(_) => Unexpected::Map,
    }
}

// use std::fmt;
// use std::marker::PhantomData;

// use serde::__private::de::{Content, ContentDeserializer};
// use serde::de::{self, IntoDeserializer, MapAccess};
// use serde::{Deserialize, de::Visitor};

pub struct FirstTagVisitor<T> {
    expecting: &'static str,
    value: PhantomData<T>,
}

impl<T> FirstTagVisitor<T> {
    /// Visitor for the content of an internally tagged enum with the given tag name.
    pub fn new(expecting: &'static str) -> Self {
        FirstTagVisitor {
            expecting,
            value: PhantomData,
        }
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for FirstTagVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = (T, Option<Content<'de>>);

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.expecting)
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: de::SeqAccess<'de>,
    {
        // Try to get the first element which should be the tag
        let tag = match seq.next_element()? {
            Some(first) => first,
            None => {
                return Err(de::Error::missing_field(
                    "tag was not found in empty sequence",
                ));
            }
        };

        // Collect the rest of the sequence (without the tag)
        let mut elements = Vec::<Content>::new();
        while let Some(elem) = seq.next_element()? {
            elements.push(elem);
        }

        // Return the tag and the sequence without the tag element
        Ok((tag, Some(Content::Seq(elements))))
    }

    // For primitive types, try to deserialize the value as T and return None for content
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::Bool(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Bool(v), &self)),
        }
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::I8(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(
                de::Unexpected::Signed(v as i64),
                &self,
            )),
        }
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::I16(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(
                de::Unexpected::Signed(v as i64),
                &self,
            )),
        }
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::I32(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(
                de::Unexpected::Signed(v as i64),
                &self,
            )),
        }
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::I64(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Signed(v), &self)),
        }
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::U8(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(
                de::Unexpected::Unsigned(v as u64),
                &self,
            )),
        }
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::U16(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(
                de::Unexpected::Unsigned(v as u64),
                &self,
            )),
        }
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::U32(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(
                de::Unexpected::Unsigned(v as u64),
                &self,
            )),
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::U64(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Unsigned(v), &self)),
        }
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::F32(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(
                de::Unexpected::Float(v as f64),
                &self,
            )),
        }
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::F64(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Float(v), &self)),
        }
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::Char(v);
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Char(v), &self)),
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::String(v.to_owned());
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Str(v), &self)),
        }
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::String(v.clone());
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Str(&v), &self)),
        }
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::ByteBuf(v.into());
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Bytes(v), &self)),
        }
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::ByteBuf(v.clone());
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Bytes(&v), &self)),
        }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::None;
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Unit, &self)),
        }
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let content = Content::deserialize(deserializer)?;
        match T::deserialize::<ContentDeserializer<'_, D::Error>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::custom("Could not deserialize `Some` as tag")),
        }
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let content = Content::Unit;
        match T::deserialize::<ContentDeserializer<'_, E>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::invalid_type(de::Unexpected::Unit, &self)),
        }
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let content = Content::deserialize(deserializer)?;
        match T::deserialize::<ContentDeserializer<'_, D::Error>>(content.into_deserializer()) {
            Ok(tag) => Ok((tag, None)),
            Err(_) => Err(de::Error::custom(
                "Could not deserialize newtype struct as tag",
            )),
        }
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(v)
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_bytes(v)
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let _ = map;
        Err(de::Error::invalid_type(de::Unexpected::Map, &self))
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let _ = data;
        Err(de::Error::invalid_type(de::Unexpected::Enum, &self))
    }
}
