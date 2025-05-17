use serde_json::json;

#[test]
fn test_basic() {
    #[derive(serde_implicit_proc::Deserialize)]
    // #[serde(untagged)]
    enum MultiTypeTag {
        StringVariant {
            #[tag]
            string_tag: String,
            value: u32,
        },
        NumberVariant {
            #[tag]
            number_tag: u64,
            value: String,
        },
        BoolVariant {
            #[tag]
            bool_tag: bool,
            value: Vec<String>,
        },
    }

    let res: Result<MultiTypeTag, _> =
        serde_json::from_value(json!({ "string_tag": "", "value": 0 }));
    // println!("{res:?}");
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
