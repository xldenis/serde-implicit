# serde-implicit

*the untagged enums you wish you had*

When building api types in Rust it is common to see serde's *untagged* enum representation get used to provide a more 'ergonomic' or 'aesthetic' API surface.

```rust
#[derive(Deserialize, Serialize)]
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

`serde-implicit` solves this problem by introducing an *implicitly* tagged enum representation. Each variant can be have a field annotated with `#[serde(tag)]`, and when that field is seen in input we can "commit" to parsing that variant, producing better error messages as a side-effect.

```
{ "content": "oops i mislabeled my field", "username": "xldenis", "timestamp": 1234 }

"missing field `sender`"
```
