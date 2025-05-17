use serde_json::json;

#[test]
fn test_basic() {
    #[derive(serde_implicit::Deserialize, Debug)]
    // #[serde(untagged)]
    enum Message {
        // { "content": "i love serializing", "sender": "xldenis", "timestamp": 123 }
        Text {
            #[tag]
            content: String,
            sender: String,
            timestamp: u64,
        },
        // { "image_url": "https://blah.com/omg.gif" }
        Image {
            #[tag]
            image_url: String,
            caption: Option<String>,
        },
        // { "emoji": "floating_man", "message_id": 123 }
        Reaction {
            #[tag]
            emoji: String,
            message_id: u64,
        },
    }

    let res: Result<Message, _> = serde_json::from_value(
        json!({ "content": "oops i mislabeled my field", "username": "xldenis", "timestamp": 1234 }),
    );
    println!("{res:?}");
    assert!(res.is_ok());

    // let res: Result<Omg, _> = serde_json::from_value(json!({"blob": 123, "other_key": 09 }));
    // println!("{res:?}");
    // assert!(res.is_ok());

    // let res: Result<Omg, _> =
    //     serde_json::from_value(json!({"unique_key": true, "missing_key": true }));
    // println!("{res:?}");

    // let res: Result<Omg2, _> =
    //     serde_json::from_value(json!({"unique_key": true, "missing_key": true }));
    // println!("{res:?}");

    // assert!(res.is_ok());
}

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

// musings on test coverage

// properties: parsing with implicit tagged <-> untagged
// properties: de(ser(x)) = x

// deserialize random combinations of valid and invalid fields for types
//  check that crash free and that it fails to deserialize

// edge cases
// - empty enum
// - duplicate tags (add check)
// - extra fields
// - missing fields
// - recursive type
