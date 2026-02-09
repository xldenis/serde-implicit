// This module is private and nothing here should be used outside of
// generated code.
//
// We will iterate on the implementation for a few releases and only have to
// worry about backward compatibility for the `untagged` and `tag` attributes
// rather than for this entire mechanism.
//
// This issue is tracking making some of this stuff public:
// https://github.com/serde-rs/serde/issues/741

use std::fmt;
use std::marker::PhantomData;

use serde::de::value::{MapDeserializer, SeqDeserializer};
use serde::de::{
    self, Deserialize, DeserializeSeed, Deserializer, EnumAccess, Expected, IgnoredAny, MapAccess,
    SeqAccess, Unexpected, Visitor,
};

/// Used from generated code to buffer the contents of the Deserializer when
/// deserializing untagged enums and internally tagged enums.
///
/// Not public API. Use serde-value instead.
#[derive(Debug, Clone)]
pub enum Content<'de> {
    Bool(bool),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),

    F32(f32),
    F64(f64),

    Char(char),
    String(String),
    Str(&'de str),
    ByteBuf(Vec<u8>),
    Bytes(&'de [u8]),

    None,
    Some(Box<Content<'de>>),

    Unit,
    Newtype(Box<Content<'de>>),
    Seq(Vec<Content<'de>>),
    Map(Vec<(Content<'de>, Content<'de>)>),
}

impl<'de> Content<'de> {
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Content::Str(x) => Some(x),
            Content::String(ref x) => Some(x),
            Content::Bytes(x) => str::from_utf8(x).ok(),
            Content::ByteBuf(ref x) => str::from_utf8(x).ok(),
            _ => None,
        }
    }

    #[cold]
    fn unexpected(&self) -> Unexpected<'_> {
        match *self {
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
}

impl<'de> Deserialize<'de> for Content<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Untagged and internally tagged enums are only supported in
        // self-describing formats.
        let visitor = ContentVisitor { value: PhantomData };
        deserializer.deserialize_any(visitor)
    }
}

impl<'de, E> de::IntoDeserializer<'de, E> for Content<'de>
where
    E: de::Error,
{
    type Deserializer = ContentDeserializer<'de, E>;

    fn into_deserializer(self) -> Self::Deserializer {
        ContentDeserializer::new(self)
    }
}

impl<'a, 'de, E> de::IntoDeserializer<'de, E> for &'a Content<'de>
where
    E: de::Error,
{
    type Deserializer = ContentRefDeserializer<'a, 'de, E>;

    fn into_deserializer(self) -> Self::Deserializer {
        ContentRefDeserializer::new(self)
    }
}

/// Used to capture data in [`Content`] from other deserializers.
/// Cannot capture externally tagged enums, `i128` and `u128`.
struct ContentVisitor<'de> {
    value: PhantomData<Content<'de>>,
}

impl<'de> ContentVisitor<'de> {
    fn new() -> Self {
        ContentVisitor { value: PhantomData }
    }
}

macro_rules! tri {
    ($e:expr) => {
        $e?
    };
}

macro_rules! map_key_integer_method {
    (owned $name:ident, $visit:ident, $ty:ty) => {
        fn $name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            match self.content {
                Content::String(ref s) => {
                    if let Ok(v) = s.parse::<$ty>() {
                        return visitor.$visit(v);
                    }
                }
                Content::Str(s) => {
                    if let Ok(v) = s.parse::<$ty>() {
                        return visitor.$visit(v);
                    }
                }
                _ => {}
            }
            ContentDeserializer::new(self.content).deserialize_integer(visitor)
        }
    };
    (ref $name:ident, $visit:ident, $ty:ty) => {
        fn $name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            match *self.content {
                Content::String(ref s) => {
                    if let Ok(v) = s.parse::<$ty>() {
                        return visitor.$visit(v);
                    }
                }
                Content::Str(s) => {
                    if let Ok(v) = s.parse::<$ty>() {
                        return visitor.$visit(v);
                    }
                }
                _ => {}
            }
            ContentRefDeserializer::new(self.content).deserialize_integer(visitor)
        }
    };
}

impl<'de> Visitor<'de> for ContentVisitor<'de> {
    type Value = Content<'de>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("any value")
    }

    fn visit_bool<F>(self, value: bool) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Bool(value))
    }

    fn visit_i8<F>(self, value: i8) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I8(value))
    }

    fn visit_i16<F>(self, value: i16) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I16(value))
    }

    fn visit_i32<F>(self, value: i32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I32(value))
    }

    fn visit_i64<F>(self, value: i64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::I64(value))
    }

    fn visit_u8<F>(self, value: u8) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U8(value))
    }

    fn visit_u16<F>(self, value: u16) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U16(value))
    }

    fn visit_u32<F>(self, value: u32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U32(value))
    }

    fn visit_u64<F>(self, value: u64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::U64(value))
    }

    fn visit_f32<F>(self, value: f32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::F32(value))
    }

    fn visit_f64<F>(self, value: f64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::F64(value))
    }

    fn visit_char<F>(self, value: char) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Char(value))
    }

    fn visit_str<F>(self, value: &str) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::String(value.into()))
    }

    fn visit_borrowed_str<F>(self, value: &'de str) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Str(value))
    }

    fn visit_string<F>(self, value: String) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::String(value))
    }

    fn visit_bytes<F>(self, value: &[u8]) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::ByteBuf(value.into()))
    }

    fn visit_borrowed_bytes<F>(self, value: &'de [u8]) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Bytes(value))
    }

    fn visit_byte_buf<F>(self, value: Vec<u8>) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::ByteBuf(value))
    }

    fn visit_unit<F>(self) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::Unit)
    }

    fn visit_none<F>(self) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        Ok(Content::None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = tri!(Deserialize::deserialize(deserializer));
        Ok(Content::Some(Box::new(v)))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = tri!(Deserialize::deserialize(deserializer));
        Ok(Content::Newtype(Box::new(v)))
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut vec = Vec::<Content>::with_capacity(visitor.size_hint().unwrap_or(0));
        while let Some(e) = tri!(visitor.next_element()) {
            vec.push(e);
        }
        Ok(Content::Seq(vec))
    }

    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut vec = Vec::<(Content, Content)>::with_capacity(visitor.size_hint().unwrap_or(0));
        while let Some(kv) = tri!(visitor.next_entry()) {
            vec.push(kv);
        }
        Ok(Content::Map(vec))
    }

    fn visit_enum<V>(self, _visitor: V) -> Result<Self::Value, V::Error>
    where
        V: EnumAccess<'de>,
    {
        Err(de::Error::custom(
            "untagged and internally tagged enums do not support enum input",
        ))
    }
}

/// This is the type of the map keys in an internally tagged enum.
///
/// Not public API.
pub enum TagOrContent<'de> {
    Tag,
    Content(Content<'de>),
}

/// Serves as a seed for deserializing a key of internally tagged enum.
/// Cannot capture externally tagged enums, `i128` and `u128`.
struct TagOrContentVisitor<'de> {
    name: &'static str,
    value: PhantomData<TagOrContent<'de>>,
}

impl<'de> TagOrContentVisitor<'de> {
    fn new(name: &'static str) -> Self {
        TagOrContentVisitor {
            name,
            value: PhantomData,
        }
    }
}

impl<'de> DeserializeSeed<'de> for TagOrContentVisitor<'de> {
    type Value = TagOrContent<'de>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Internally tagged enums are only supported in self-describing
        // formats.
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for TagOrContentVisitor<'de> {
    type Value = TagOrContent<'de>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "a type tag `{}` or any other value", self.name)
    }

    fn visit_bool<F>(self, value: bool) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_bool(value)
            .map(TagOrContent::Content)
    }

    fn visit_i8<F>(self, value: i8) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_i8(value)
            .map(TagOrContent::Content)
    }

    fn visit_i16<F>(self, value: i16) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_i16(value)
            .map(TagOrContent::Content)
    }

    fn visit_i32<F>(self, value: i32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_i32(value)
            .map(TagOrContent::Content)
    }

    fn visit_i64<F>(self, value: i64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_i64(value)
            .map(TagOrContent::Content)
    }

    fn visit_u8<F>(self, value: u8) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_u8(value)
            .map(TagOrContent::Content)
    }

    fn visit_u16<F>(self, value: u16) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_u16(value)
            .map(TagOrContent::Content)
    }

    fn visit_u32<F>(self, value: u32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_u32(value)
            .map(TagOrContent::Content)
    }

    fn visit_u64<F>(self, value: u64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_u64(value)
            .map(TagOrContent::Content)
    }

    fn visit_f32<F>(self, value: f32) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_f32(value)
            .map(TagOrContent::Content)
    }

    fn visit_f64<F>(self, value: f64) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_f64(value)
            .map(TagOrContent::Content)
    }

    fn visit_char<F>(self, value: char) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_char(value)
            .map(TagOrContent::Content)
    }

    fn visit_str<F>(self, value: &str) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == self.name {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor::new()
                .visit_str(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_borrowed_str<F>(self, value: &'de str) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == self.name {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor::new()
                .visit_borrowed_str(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_string<F>(self, value: String) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == self.name {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor::new()
                .visit_string(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_bytes<F>(self, value: &[u8]) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == self.name.as_bytes() {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor::new()
                .visit_bytes(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_borrowed_bytes<F>(self, value: &'de [u8]) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == self.name.as_bytes() {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor::new()
                .visit_borrowed_bytes(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_byte_buf<F>(self, value: Vec<u8>) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        if value == self.name.as_bytes() {
            Ok(TagOrContent::Tag)
        } else {
            ContentVisitor::new()
                .visit_byte_buf(value)
                .map(TagOrContent::Content)
        }
    }

    fn visit_unit<F>(self) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_unit()
            .map(TagOrContent::Content)
    }

    fn visit_none<F>(self) -> Result<Self::Value, F>
    where
        F: de::Error,
    {
        ContentVisitor::new()
            .visit_none()
            .map(TagOrContent::Content)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        ContentVisitor::new()
            .visit_some(deserializer)
            .map(TagOrContent::Content)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        ContentVisitor::new()
            .visit_newtype_struct(deserializer)
            .map(TagOrContent::Content)
    }

    fn visit_seq<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        ContentVisitor::new()
            .visit_seq(visitor)
            .map(TagOrContent::Content)
    }

    fn visit_map<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        ContentVisitor::new()
            .visit_map(visitor)
            .map(TagOrContent::Content)
    }

    fn visit_enum<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: EnumAccess<'de>,
    {
        ContentVisitor::new()
            .visit_enum(visitor)
            .map(TagOrContent::Content)
    }
}

/// Used by generated code to deserialize an internally tagged enum.
///
/// Captures map or sequence from the original deserializer and searches
/// a tag in it (in case of sequence, tag is the first element of sequence).
///
/// Not public API.
pub struct TaggedContentVisitor<T> {
    tag_name: &'static str,
    expecting: &'static str,
    value: PhantomData<T>,
}

impl<T> TaggedContentVisitor<T> {
    /// Visitor for the content of an internally tagged enum with the given
    /// tag name.
    pub fn new(name: &'static str, expecting: &'static str) -> Self {
        TaggedContentVisitor {
            tag_name: name,
            expecting,
            value: PhantomData,
        }
    }
}

impl<'de, T> Visitor<'de> for TaggedContentVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = (T, Content<'de>);

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.expecting)
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let tag = match tri!(seq.next_element()) {
            Some(tag) => tag,
            None => {
                return Err(de::Error::missing_field(self.tag_name));
            }
        };
        let rest = de::value::SeqAccessDeserializer::new(seq);
        Ok((tag, tri!(Content::deserialize(rest))))
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut tag = None;
        let mut vec = Vec::<(Content, Content)>::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(k) = tri!(map.next_key_seed(TagOrContentVisitor::new(self.tag_name))) {
            match k {
                TagOrContent::Tag => {
                    if tag.is_some() {
                        return Err(de::Error::duplicate_field(self.tag_name));
                    }
                    tag = Some(tri!(map.next_value()));
                }
                TagOrContent::Content(k) => {
                    let v = tri!(map.next_value());
                    vec.push((k, v));
                }
            }
        }
        match tag {
            None => Err(de::Error::missing_field(self.tag_name)),
            Some(tag) => Ok((tag, Content::Map(vec))),
        }
    }
}

/// Used by generated code to deserialize an adjacently tagged enum.
///
/// Not public API.
pub enum TagOrContentField {
    Tag,
    Content,
}

/// Not public API.
pub struct TagOrContentFieldVisitor {
    /// Name of the tag field of the adjacently tagged enum
    pub tag: &'static str,
    /// Name of the content field of the adjacently tagged enum
    pub content: &'static str,
}

impl<'de> DeserializeSeed<'de> for TagOrContentFieldVisitor {
    type Value = TagOrContentField;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(self)
    }
}

impl<'de> Visitor<'de> for TagOrContentFieldVisitor {
    type Value = TagOrContentField;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{:?} or {:?}", self.tag, self.content)
    }

    fn visit_u64<E>(self, field_index: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match field_index {
            0 => Ok(TagOrContentField::Tag),
            1 => Ok(TagOrContentField::Content),
            _ => Err(de::Error::invalid_value(
                Unexpected::Unsigned(field_index),
                &self,
            )),
        }
    }

    fn visit_str<E>(self, field: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if field == self.tag {
            Ok(TagOrContentField::Tag)
        } else if field == self.content {
            Ok(TagOrContentField::Content)
        } else {
            Err(de::Error::invalid_value(Unexpected::Str(field), &self))
        }
    }

    fn visit_bytes<E>(self, field: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if field == self.tag.as_bytes() {
            Ok(TagOrContentField::Tag)
        } else if field == self.content.as_bytes() {
            Ok(TagOrContentField::Content)
        } else {
            Err(de::Error::invalid_value(Unexpected::Bytes(field), &self))
        }
    }
}

/// Used by generated code to deserialize an adjacently tagged enum when
/// ignoring unrelated fields is allowed.
///
/// Not public API.
pub enum TagContentOtherField {
    Tag,
    Content,
    Other,
}

/// Not public API.
pub struct TagContentOtherFieldVisitor {
    /// Name of the tag field of the adjacently tagged enum
    pub tag: &'static str,
    /// Name of the content field of the adjacently tagged enum
    pub content: &'static str,
}

impl<'de> DeserializeSeed<'de> for TagContentOtherFieldVisitor {
    type Value = TagContentOtherField;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(self)
    }
}

impl<'de> Visitor<'de> for TagContentOtherFieldVisitor {
    type Value = TagContentOtherField;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{:?}, {:?}, or other ignored fields",
            self.tag, self.content
        )
    }

    fn visit_u64<E>(self, field_index: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match field_index {
            0 => Ok(TagContentOtherField::Tag),
            1 => Ok(TagContentOtherField::Content),
            _ => Ok(TagContentOtherField::Other),
        }
    }

    fn visit_str<E>(self, field: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_bytes(field.as_bytes())
    }

    fn visit_bytes<E>(self, field: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if field == self.tag.as_bytes() {
            Ok(TagContentOtherField::Tag)
        } else if field == self.content.as_bytes() {
            Ok(TagContentOtherField::Content)
        } else {
            Ok(TagContentOtherField::Other)
        }
    }
}

/// Not public API
pub struct ContentDeserializer<'de, E> {
    content: Content<'de>,
    err: PhantomData<E>,
}

impl<'de, E> ContentDeserializer<'de, E>
where
    E: de::Error,
{
    #[cold]
    fn invalid_type(self, exp: &dyn Expected) -> E {
        de::Error::invalid_type(self.content.unexpected(), exp)
    }

    fn deserialize_integer<V>(self, visitor: V) -> Result<V::Value, E>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::U8(v) => visitor.visit_u8(v),
            Content::U16(v) => visitor.visit_u16(v),
            Content::U32(v) => visitor.visit_u32(v),
            Content::U64(v) => visitor.visit_u64(v),
            Content::I8(v) => visitor.visit_i8(v),
            Content::I16(v) => visitor.visit_i16(v),
            Content::I32(v) => visitor.visit_i32(v),
            Content::I64(v) => visitor.visit_i64(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_float<V>(self, visitor: V) -> Result<V::Value, E>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::F32(v) => visitor.visit_f32(v),
            Content::F64(v) => visitor.visit_f64(v),
            Content::U8(v) => visitor.visit_u8(v),
            Content::U16(v) => visitor.visit_u16(v),
            Content::U32(v) => visitor.visit_u32(v),
            Content::U64(v) => visitor.visit_u64(v),
            Content::I8(v) => visitor.visit_i8(v),
            Content::I16(v) => visitor.visit_i16(v),
            Content::I32(v) => visitor.visit_i32(v),
            Content::I64(v) => visitor.visit_i64(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
}

fn visit_content_seq<'de, V, E>(content: Vec<Content<'de>>, visitor: V) -> Result<V::Value, E>
where
    V: Visitor<'de>,
    E: de::Error,
{
    let mut seq_visitor = SeqDeserializer::new(content.into_iter());
    let value = tri!(visitor.visit_seq(&mut seq_visitor));
    tri!(seq_visitor.end());
    Ok(value)
}

fn visit_content_map<'de, V, E>(
    content: Vec<(Content<'de>, Content<'de>)>,
    visitor: V,
) -> Result<V::Value, E>
where
    V: Visitor<'de>,
    E: de::Error,
{
    let mut map_visitor =
        MapDeserializer::new(content.into_iter().map(|(k, v)| (MapKeyContent(k), v)));
    let value = tri!(visitor.visit_map(&mut map_visitor));
    tri!(map_visitor.end());
    Ok(value)
}

/// Used when deserializing an internally tagged enum because the content
/// will be used exactly once.
impl<'de, E> Deserializer<'de> for ContentDeserializer<'de, E>
where
    E: de::Error,
{
    type Error = E;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::Bool(v) => visitor.visit_bool(v),
            Content::U8(v) => visitor.visit_u8(v),
            Content::U16(v) => visitor.visit_u16(v),
            Content::U32(v) => visitor.visit_u32(v),
            Content::U64(v) => visitor.visit_u64(v),
            Content::I8(v) => visitor.visit_i8(v),
            Content::I16(v) => visitor.visit_i16(v),
            Content::I32(v) => visitor.visit_i32(v),
            Content::I64(v) => visitor.visit_i64(v),
            Content::F32(v) => visitor.visit_f32(v),
            Content::F64(v) => visitor.visit_f64(v),
            Content::Char(v) => visitor.visit_char(v),
            Content::String(v) => visitor.visit_string(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(v) => visitor.visit_byte_buf(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            Content::Unit => visitor.visit_unit(),
            Content::None => visitor.visit_none(),
            Content::Some(v) => visitor.visit_some(ContentDeserializer::new(*v)),
            Content::Newtype(v) => visitor.visit_newtype_struct(ContentDeserializer::new(*v)),
            Content::Seq(v) => visit_content_seq(v, visitor),
            Content::Map(v) => visit_content_map(v, visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::Bool(v) => visitor.visit_bool(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_float(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_float(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::Char(v) => visitor.visit_char(v),
            Content::String(v) => visitor.visit_string(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::String(v) => visitor.visit_string(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(v) => visitor.visit_byte_buf(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::String(v) => visitor.visit_string(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(v) => visitor.visit_byte_buf(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            Content::Seq(v) => visit_content_seq(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::None => visitor.visit_none(),
            Content::Some(v) => visitor.visit_some(ContentDeserializer::new(*v)),
            Content::Unit => visitor.visit_unit(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::Unit => visitor.visit_unit(),

            // Allow deserializing newtype variant containing unit.
            //
            //     #[derive(Deserialize)]
            //     #[serde(tag = "result")]
            //     enum Response<T> {
            //         Success(T),
            //     }
            //
            // We want {"result":"Success"} to deserialize into Response<()>.
            Content::Map(ref v) if v.is_empty() => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            // As a special case, allow deserializing untagged newtype
            // variant containing unit struct.
            //
            //     #[derive(Deserialize)]
            //     struct Info;
            //
            //     #[derive(Deserialize)]
            //     #[serde(tag = "topic")]
            //     enum Message {
            //         Info(Info),
            //     }
            //
            // We want {"topic":"Info"} to deserialize even though
            // ordinarily unit structs do not deserialize from empty map/seq.
            Content::Map(ref v) if v.is_empty() => visitor.visit_unit(),
            Content::Seq(ref v) if v.is_empty() => visitor.visit_unit(),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_newtype_struct<V>(self, _name: &str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::Newtype(v) => visitor.visit_newtype_struct(ContentDeserializer::new(*v)),
            _ => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::Seq(v) => visit_content_seq(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::Map(v) => visit_content_map(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::Seq(v) => visit_content_seq(v, visitor),
            Content::Map(v) => visit_content_map(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let (variant, value) = match self.content {
            Content::Map(value) => {
                let mut iter = value.into_iter();
                let (variant, value) = match iter.next() {
                    Some(v) => v,
                    None => {
                        return Err(de::Error::invalid_value(
                            de::Unexpected::Map,
                            &"map with a single key",
                        ));
                    }
                };
                // enums are encoded in json as maps with a single key:value pair
                if iter.next().is_some() {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Map,
                        &"map with a single key",
                    ));
                }
                (variant, Some(value))
            }
            s @ Content::String(_) | s @ Content::Str(_) => (s, None),
            other => {
                return Err(de::Error::invalid_type(
                    other.unexpected(),
                    &"string or map",
                ));
            }
        };

        visitor.visit_enum(EnumDeserializer::new(variant, value))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::String(v) => visitor.visit_string(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(v) => visitor.visit_byte_buf(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            Content::U8(v) => visitor.visit_u8(v),
            Content::U64(v) => visitor.visit_u64(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }
}

impl<'de, E> ContentDeserializer<'de, E> {
    /// private API, don't use
    pub fn new(content: Content<'de>) -> Self {
        ContentDeserializer {
            content,
            err: PhantomData,
        }
    }
}

struct MapKeyContent<'de>(Content<'de>);

impl<'de, E> de::IntoDeserializer<'de, E> for MapKeyContent<'de>
where
    E: de::Error,
{
    type Deserializer = ContentMapKeyDeserializer<'de, E>;

    fn into_deserializer(self) -> Self::Deserializer {
        ContentMapKeyDeserializer {
            content: self.0,
            err: PhantomData,
        }
    }
}

struct ContentMapKeyDeserializer<'de, E> {
    content: Content<'de>,
    err: PhantomData<E>,
}

impl<'de, E> Deserializer<'de> for ContentMapKeyDeserializer<'de, E>
where
    E: de::Error,
{
    type Error = E;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ContentDeserializer::new(self.content).deserialize_any(visitor)
    }

    map_key_integer_method!(owned deserialize_i8, visit_i8, i8);
    map_key_integer_method!(owned deserialize_i16, visit_i16, i16);
    map_key_integer_method!(owned deserialize_i32, visit_i32, i32);
    map_key_integer_method!(owned deserialize_i64, visit_i64, i64);
    map_key_integer_method!(owned deserialize_u8, visit_u8, u8);
    map_key_integer_method!(owned deserialize_u16, visit_u16, u16);
    map_key_integer_method!(owned deserialize_u32, visit_u32, u32);
    map_key_integer_method!(owned deserialize_u64, visit_u64, u64);

    fn deserialize_newtype_struct<V>(
        self,
        _name: &str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.content {
            Content::Newtype(v) => visitor.visit_newtype_struct(ContentMapKeyDeserializer {
                content: *v,
                err: PhantomData,
            }),
            _ => visitor.visit_newtype_struct(self),
        }
    }

    serde::forward_to_deserialize_any! {
        bool f32 f64 char str string bytes byte_buf option unit unit_struct
        seq tuple tuple_struct map struct enum identifier
        ignored_any
    }
}

pub struct EnumDeserializer<'de, E>
where
    E: de::Error,
{
    variant: Content<'de>,
    value: Option<Content<'de>>,
    err: PhantomData<E>,
}

impl<'de, E> EnumDeserializer<'de, E>
where
    E: de::Error,
{
    pub fn new(variant: Content<'de>, value: Option<Content<'de>>) -> EnumDeserializer<'de, E> {
        EnumDeserializer {
            variant,
            value,
            err: PhantomData,
        }
    }
}

impl<'de, E> de::EnumAccess<'de> for EnumDeserializer<'de, E>
where
    E: de::Error,
{
    type Error = E;
    type Variant = VariantDeserializer<'de, Self::Error>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), E>
    where
        V: de::DeserializeSeed<'de>,
    {
        let visitor = VariantDeserializer {
            value: self.value,
            err: PhantomData,
        };
        seed.deserialize(ContentDeserializer::new(self.variant))
            .map(|v| (v, visitor))
    }
}

pub struct VariantDeserializer<'de, E>
where
    E: de::Error,
{
    value: Option<Content<'de>>,
    err: PhantomData<E>,
}

impl<'de, E> de::VariantAccess<'de> for VariantDeserializer<'de, E>
where
    E: de::Error,
{
    type Error = E;

    fn unit_variant(self) -> Result<(), E> {
        match self.value {
            Some(value) => de::Deserialize::deserialize(ContentDeserializer::new(value)),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, E>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => seed.deserialize(ContentDeserializer::new(value)),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            Some(Content::Seq(v)) => {
                de::Deserializer::deserialize_any(SeqDeserializer::new(v.into_iter()), visitor)
            }
            Some(other) => Err(de::Error::invalid_type(
                other.unexpected(),
                &"tuple variant",
            )),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            Some(Content::Map(v)) => {
                de::Deserializer::deserialize_any(MapDeserializer::new(v.into_iter()), visitor)
            }
            Some(Content::Seq(v)) => {
                de::Deserializer::deserialize_any(SeqDeserializer::new(v.into_iter()), visitor)
            }
            Some(other) => Err(de::Error::invalid_type(
                other.unexpected(),
                &"struct variant",
            )),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

/// Not public API.
pub struct ContentRefDeserializer<'a, 'de: 'a, E> {
    content: &'a Content<'de>,
    err: PhantomData<E>,
}

impl<'a, 'de, E> ContentRefDeserializer<'a, 'de, E>
where
    E: de::Error,
{
    #[cold]
    fn invalid_type(self, exp: &dyn Expected) -> E {
        de::Error::invalid_type(self.content.unexpected(), exp)
    }

    fn deserialize_integer<V>(self, visitor: V) -> Result<V::Value, E>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::U8(v) => visitor.visit_u8(v),
            Content::U16(v) => visitor.visit_u16(v),
            Content::U32(v) => visitor.visit_u32(v),
            Content::U64(v) => visitor.visit_u64(v),
            Content::I8(v) => visitor.visit_i8(v),
            Content::I16(v) => visitor.visit_i16(v),
            Content::I32(v) => visitor.visit_i32(v),
            Content::I64(v) => visitor.visit_i64(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_float<V>(self, visitor: V) -> Result<V::Value, E>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::F32(v) => visitor.visit_f32(v),
            Content::F64(v) => visitor.visit_f64(v),
            Content::U8(v) => visitor.visit_u8(v),
            Content::U16(v) => visitor.visit_u16(v),
            Content::U32(v) => visitor.visit_u32(v),
            Content::U64(v) => visitor.visit_u64(v),
            Content::I8(v) => visitor.visit_i8(v),
            Content::I16(v) => visitor.visit_i16(v),
            Content::I32(v) => visitor.visit_i32(v),
            Content::I64(v) => visitor.visit_i64(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }
}

fn visit_content_seq_ref<'a, 'de, V, E>(
    content: &'a [Content<'de>],
    visitor: V,
) -> Result<V::Value, E>
where
    V: Visitor<'de>,
    E: de::Error,
{
    let mut seq_visitor = SeqDeserializer::new(content.iter());
    let value = tri!(visitor.visit_seq(&mut seq_visitor));
    tri!(seq_visitor.end());
    Ok(value)
}

fn visit_content_map_ref<'a, 'de, V, E>(
    content: &'a [(Content<'de>, Content<'de>)],
    visitor: V,
) -> Result<V::Value, E>
where
    V: Visitor<'de>,
    E: de::Error,
{
    let map = content
        .iter()
        .map(|(k, v)| (MapKeyContentRef(k), &*v));
    let mut map_visitor = MapDeserializer::new(map);
    let value = tri!(visitor.visit_map(&mut map_visitor));
    tri!(map_visitor.end());
    Ok(value)
}

/// Used when deserializing an untagged enum because the content may need
/// to be used more than once.
impl<'de, 'a, E> Deserializer<'de> for ContentRefDeserializer<'a, 'de, E>
where
    E: de::Error,
{
    type Error = E;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, E>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::Bool(v) => visitor.visit_bool(v),
            Content::U8(v) => visitor.visit_u8(v),
            Content::U16(v) => visitor.visit_u16(v),
            Content::U32(v) => visitor.visit_u32(v),
            Content::U64(v) => visitor.visit_u64(v),
            Content::I8(v) => visitor.visit_i8(v),
            Content::I16(v) => visitor.visit_i16(v),
            Content::I32(v) => visitor.visit_i32(v),
            Content::I64(v) => visitor.visit_i64(v),
            Content::F32(v) => visitor.visit_f32(v),
            Content::F64(v) => visitor.visit_f64(v),
            Content::Char(v) => visitor.visit_char(v),
            Content::String(ref v) => visitor.visit_str(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(ref v) => visitor.visit_bytes(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            Content::Unit => visitor.visit_unit(),
            Content::None => visitor.visit_none(),
            Content::Some(ref v) => visitor.visit_some(ContentRefDeserializer::new(v)),
            Content::Newtype(ref v) => visitor.visit_newtype_struct(ContentRefDeserializer::new(v)),
            Content::Seq(ref v) => visit_content_seq_ref(v, visitor),
            Content::Map(ref v) => visit_content_map_ref(v, visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::Bool(v) => visitor.visit_bool(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_float(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_float(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::Char(v) => visitor.visit_char(v),
            Content::String(ref v) => visitor.visit_str(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::String(ref v) => visitor.visit_str(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(ref v) => visitor.visit_bytes(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::String(ref v) => visitor.visit_str(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(ref v) => visitor.visit_bytes(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            Content::Seq(ref v) => visit_content_seq_ref(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, E>
    where
        V: Visitor<'de>,
    {
        // Covered by tests/test_enum_untagged.rs
        //      with_optional_field::*
        match *self.content {
            Content::None => visitor.visit_none(),
            Content::Some(ref v) => visitor.visit_some(ContentRefDeserializer::new(v)),
            Content::Unit => visitor.visit_unit(),
            // This case is to support data formats which do not encode an
            // indication whether a value is optional. An example of such a
            // format is JSON, and a counterexample is RON. When requesting
            // `deserialize_any` in JSON, the data format never performs
            // `Visitor::visit_some` but we still must be able to
            // deserialize the resulting Content into data structures with
            // optional fields.
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::Unit => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &str, visitor: V) -> Result<V::Value, E>
    where
        V: Visitor<'de>,
    {
        // Covered by tests/test_enum_untagged.rs
        //      newtype_struct
        match *self.content {
            Content::Newtype(ref v) => visitor.visit_newtype_struct(ContentRefDeserializer::new(v)),
            // This case is to support data formats that encode newtype
            // structs and their underlying data the same, with no
            // indication whether a newtype wrapper was present. For example
            // JSON does this, while RON does not. In RON a newtype's name
            // is included in the serialized representation and it knows to
            // call `Visitor::visit_newtype_struct` from `deserialize_any`.
            // JSON's `deserialize_any` never calls `visit_newtype_struct`
            // but in this code we still must be able to deserialize the
            // resulting Content into newtypes.
            _ => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::Seq(ref v) => visit_content_seq_ref(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::Map(ref v) => visit_content_map_ref(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::Seq(ref v) => visit_content_seq_ref(v, visitor),
            Content::Map(ref v) => visit_content_map_ref(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let (variant, value) = match *self.content {
            Content::Map(ref value) => {
                let mut iter = value.iter();
                let (variant, value) = match iter.next() {
                    Some(v) => v,
                    None => {
                        return Err(de::Error::invalid_value(
                            de::Unexpected::Map,
                            &"map with a single key",
                        ));
                    }
                };
                // enums are encoded in json as maps with a single key:value pair
                if iter.next().is_some() {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Map,
                        &"map with a single key",
                    ));
                }
                (variant, Some(value))
            }
            ref s @ Content::String(_) | ref s @ Content::Str(_) => (s, None),
            ref other => {
                return Err(de::Error::invalid_type(
                    other.unexpected(),
                    &"string or map",
                ));
            }
        };

        visitor.visit_enum(EnumRefDeserializer {
            variant,
            value,
            err: PhantomData,
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::String(ref v) => visitor.visit_str(v),
            Content::Str(v) => visitor.visit_borrowed_str(v),
            Content::ByteBuf(ref v) => visitor.visit_bytes(v),
            Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
            Content::U8(v) => visitor.visit_u8(v),
            Content::U64(v) => visitor.visit_u64(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

impl<'a, 'de, E> ContentRefDeserializer<'a, 'de, E> {
    /// private API, don't use
    pub fn new(content: &'a Content<'de>) -> Self {
        ContentRefDeserializer {
            content,
            err: PhantomData,
        }
    }
}

impl<'a, 'de: 'a, E> Copy for ContentRefDeserializer<'a, 'de, E> {}

impl<'a, 'de: 'a, E> Clone for ContentRefDeserializer<'a, 'de, E> {
    fn clone(&self) -> Self {
        *self
    }
}

struct MapKeyContentRef<'a, 'de: 'a>(&'a Content<'de>);

impl<'a, 'de, E> de::IntoDeserializer<'de, E> for MapKeyContentRef<'a, 'de>
where
    E: de::Error,
{
    type Deserializer = ContentRefMapKeyDeserializer<'a, 'de, E>;

    fn into_deserializer(self) -> Self::Deserializer {
        ContentRefMapKeyDeserializer {
            content: self.0,
            err: PhantomData,
        }
    }
}

struct ContentRefMapKeyDeserializer<'a, 'de: 'a, E> {
    content: &'a Content<'de>,
    err: PhantomData<E>,
}

impl<'a, 'de: 'a, E> Copy for ContentRefMapKeyDeserializer<'a, 'de, E> {}

impl<'a, 'de: 'a, E> Clone for ContentRefMapKeyDeserializer<'a, 'de, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'de, 'a, E> Deserializer<'de> for ContentRefMapKeyDeserializer<'a, 'de, E>
where
    E: de::Error,
{
    type Error = E;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ContentRefDeserializer::new(self.content).deserialize_any(visitor)
    }

    map_key_integer_method!(ref deserialize_i8, visit_i8, i8);
    map_key_integer_method!(ref deserialize_i16, visit_i16, i16);
    map_key_integer_method!(ref deserialize_i32, visit_i32, i32);
    map_key_integer_method!(ref deserialize_i64, visit_i64, i64);
    map_key_integer_method!(ref deserialize_u8, visit_u8, u8);
    map_key_integer_method!(ref deserialize_u16, visit_u16, u16);
    map_key_integer_method!(ref deserialize_u32, visit_u32, u32);
    map_key_integer_method!(ref deserialize_u64, visit_u64, u64);

    fn deserialize_newtype_struct<V>(
        self,
        _name: &str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match *self.content {
            Content::Newtype(ref v) => visitor.visit_newtype_struct(ContentRefMapKeyDeserializer {
                content: v,
                err: PhantomData,
            }),
            _ => visitor.visit_newtype_struct(self),
        }
    }

    serde::forward_to_deserialize_any! {
        bool f32 f64 char str string bytes byte_buf option unit unit_struct
        seq tuple tuple_struct map struct enum identifier
        ignored_any
    }
}

struct EnumRefDeserializer<'a, 'de: 'a, E>
where
    E: de::Error,
{
    variant: &'a Content<'de>,
    value: Option<&'a Content<'de>>,
    err: PhantomData<E>,
}

impl<'de, 'a, E> de::EnumAccess<'de> for EnumRefDeserializer<'a, 'de, E>
where
    E: de::Error,
{
    type Error = E;
    type Variant = VariantRefDeserializer<'a, 'de, Self::Error>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let visitor = VariantRefDeserializer {
            value: self.value,
            err: PhantomData,
        };
        seed.deserialize(ContentRefDeserializer::new(self.variant))
            .map(|v| (v, visitor))
    }
}

struct VariantRefDeserializer<'a, 'de: 'a, E>
where
    E: de::Error,
{
    value: Option<&'a Content<'de>>,
    err: PhantomData<E>,
}

impl<'de, 'a, E> de::VariantAccess<'de> for VariantRefDeserializer<'a, 'de, E>
where
    E: de::Error,
{
    type Error = E;

    fn unit_variant(self) -> Result<(), E> {
        match self.value {
            Some(value) => de::Deserialize::deserialize(ContentRefDeserializer::new(value)),
            // Covered by tests/test_annotations.rs
            //      test_partially_untagged_adjacently_tagged_enum
            // Covered by tests/test_enum_untagged.rs
            //      newtype_enum::unit
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, E>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.value {
            // Covered by tests/test_annotations.rs
            //      test_partially_untagged_enum_desugared
            //      test_partially_untagged_enum_generic
            // Covered by tests/test_enum_untagged.rs
            //      newtype_enum::newtype
            Some(value) => seed.deserialize(ContentRefDeserializer::new(value)),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            // Covered by tests/test_annotations.rs
            //      test_partially_untagged_enum
            //      test_partially_untagged_enum_desugared
            // Covered by tests/test_enum_untagged.rs
            //      newtype_enum::tuple0
            //      newtype_enum::tuple2
            Some(Content::Seq(v)) => visit_content_seq_ref(v, visitor),
            Some(other) => Err(de::Error::invalid_type(
                other.unexpected(),
                &"tuple variant",
            )),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            // Covered by tests/test_enum_untagged.rs
            //      newtype_enum::struct_from_map
            Some(Content::Map(v)) => visit_content_map_ref(v, visitor),
            // Covered by tests/test_enum_untagged.rs
            //      newtype_enum::struct_from_seq
            //      newtype_enum::empty_struct_from_seq
            Some(Content::Seq(v)) => visit_content_seq_ref(v, visitor),
            Some(other) => Err(de::Error::invalid_type(
                other.unexpected(),
                &"struct variant",
            )),
            None => Err(de::Error::invalid_type(
                de::Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

impl<'de, E> de::IntoDeserializer<'de, E> for ContentDeserializer<'de, E>
where
    E: de::Error,
{
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

impl<'de, 'a, E> de::IntoDeserializer<'de, E> for ContentRefDeserializer<'a, 'de, E>
where
    E: de::Error,
{
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

/// Visitor for deserializing an internally tagged unit variant.
///
/// Not public API.
pub struct InternallyTaggedUnitVisitor<'a> {
    type_name: &'a str,
    variant_name: &'a str,
}

impl<'a> InternallyTaggedUnitVisitor<'a> {
    /// Not public API.
    pub fn new(type_name: &'a str, variant_name: &'a str) -> Self {
        InternallyTaggedUnitVisitor {
            type_name,
            variant_name,
        }
    }
}

impl<'de, 'a> Visitor<'de> for InternallyTaggedUnitVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "unit variant {}::{}",
            self.type_name, self.variant_name
        )
    }

    fn visit_seq<S>(self, _: S) -> Result<(), S::Error>
    where
        S: SeqAccess<'de>,
    {
        Ok(())
    }

    fn visit_map<M>(self, mut access: M) -> Result<(), M::Error>
    where
        M: MapAccess<'de>,
    {
        while tri!(access.next_entry::<IgnoredAny, IgnoredAny>()).is_some() {}
        Ok(())
    }
}

/// Visitor for deserializing an untagged unit variant.
///
/// Not public API.
pub struct UntaggedUnitVisitor<'a> {
    type_name: &'a str,
    variant_name: &'a str,
}

impl<'a> UntaggedUnitVisitor<'a> {
    /// Not public API.
    pub fn new(type_name: &'a str, variant_name: &'a str) -> Self {
        UntaggedUnitVisitor {
            type_name,
            variant_name,
        }
    }
}

impl<'de, 'a> Visitor<'de> for UntaggedUnitVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "unit variant {}::{}",
            self.type_name, self.variant_name
        )
    }

    fn visit_unit<E>(self) -> Result<(), E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<(), E>
    where
        E: de::Error,
    {
        Ok(())
    }
}
