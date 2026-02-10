# serde-implicit

*the untagged enums you wish you had*

When building api types in Rust it is common to see serde's *untagged* enum representation get used to provide a more 'ergonomic' or 'aesthetic' API surface.

```rust
#[derive(serde_implicit::Deserialize, Serialize)]
#[serde(untagged)]
enum Message {
	// { "content": "i love serializing", "sender": "xldenis", "timestamp": 123 }
    Text { content: String, sender: String, timestamp: u64, },
	// { "image_url": "https://blah.com/omg.gif" }
    Image { image_url: String, caption: Option<String>, },
	// { "emoji": "floating_man", "message_id": 123 }
    Reaction { emoji: String, message_id: u64, },
}
```

This approach has one major downside: absolutely garbage error messages.
Even when your enum types have completely disjoint fields, serde will blindly attempt to parse all variants. A single missing or extra field leads to the dreaded:

```
{ "content": "oops i mislabeled my field", "username": "xldenis", "timestamp": 1234 }

"data did not match any variant of untagged enum Message"
```

`serde-implicit` solves this problem by introducing an *implicitly* tagged enum representation. Each variant can be have a field annotated with `#[serde_implicit(tag)]`, and when that field is seen in input we can "commit" to parsing that variant, producing better error messages as a side-effect.

**Important:** Tag fields should be non-optional (not `Option<T>`). During deserialization, `null` values are ignored when searching for the implicit tag — a field with a `null` value will not be used to select a variant. This means if your tag field is `Option<T>` and the input contains `"tag_field": null`, that variant will not be matched.

```
{ "content": "oops i mislabeled my field", "username": "xldenis", "timestamp": 1234 }

"missing field `sender`"
```

## Tuple variant support

`serde-implicit` also provides support for tuple variants, allowing you to use a specific field position as the tag of the enum. Variants are scanned top-down, checking only the tag fields at first. As soon as a tag is matched, that variant is *locked in* and the complete set of fields is then parsed. This allows providing better error messages than *untagged* enums like them comes with several tradeoffs. In particular `serde-implicit` is not able to provide the same level of overlap-checking that is achievable with struct enums, meaning it is possible to have unreachable variants.

**note:** tuple enums are *only* parsed as sequences `[field1, field2, field3]`, serde-json's object-syntax for tuples is not supported.

```rust
#[derive(serde_implicit::Deserialize, Serialize)]
#[serde(untagged)]
enum Message {
    Literal(u64),
    BigOp(Op, Vec<Message>),
}

#[derive(serde::Deserialize, Debug)]
enum Op {
    Sum,
}
```

With `serde`, and `untagged` if you tried to parse the message `["Sum", 1]` you would get the following error:

```
data did not match any variant of untagged enum Message
```

With `serde-implicit`, you would get:

```
invalid type: integer `1`, expected a sequence
```