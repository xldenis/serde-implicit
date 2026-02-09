use serde_json::json;

#[test]
fn test_basic() {
    #[allow(dead_code)]
    #[derive(serde_implicit_proc::Deserialize, Debug)]
    // #[serde(untagged)]
    enum MultiTypeTag {
        StringVariant {
            #[serde_implicit(tag)]
            string_tag: String,
            value: u32,
        },
        NumberVariant {
            #[serde_implicit(tag)]
            number_tag: u64,
            value: String,
            unique_field: String,
        },
        BoolVariant {
            #[serde_implicit(tag)]
            bool_tag: bool,
            value: Vec<String>,
        },
    }

    let res: Result<MultiTypeTag, _> =
        serde_json::from_value(json!({ "string_tag": "", "value": 0 }));
    assert!(res.is_ok());

    let res: Result<MultiTypeTag, _> =
        serde_json::from_value(json!({ "string_tag": "", "value": 0, "extra_field": "1234" }));
    assert!(res.is_ok());

    let res: Result<MultiTypeTag, _> =
        serde_json::from_value(json!({ "string_tag": "", "value": "straing" }));

    let err = res.unwrap_err();
    assert!(
        matches!(
            &*err.to_string(),
            r#"invalid type: string "straing", expected u32"#
        ),
        "{err}",
    );

    let res: Result<MultiTypeTag, _> = serde_json::from_value(json!({ "string_tag": "" }));

    let err = res.unwrap_err();
    assert!(
        matches!(&*err.to_string(), r#"missing field `value`"#),
        "{err}",
    );

    // output specific error message about `unique_field` (if constructor is `deny_unknown_fields`)
    // let res: Result<MultiTypeTag, _> =
    //     serde_json::from_value(json!({ "string_tag": "", "unique_field": "" }));

    // let err = res.unwrap_err();
    // assert!(
    //     matches!(&*err.to_string(), r#"missing field `value`"#),
    //     "{err}",
    // );
}

#[test]
fn tuple_basic() {
    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum TupleEnum {
        Case1(bool, u32),
        Case2(u32),
    }

    let res: Result<TupleEnum, _> = serde_json::from_value(json!([true, 0]));
    assert!(res.is_ok());

    let res: Result<TupleEnum, _> = serde_json::from_value(json!([0]));

    assert_eq!(res.unwrap(), TupleEnum::Case2(0));
}

#[test]
fn tuple_overlap() {
    // Because `serde-implicit` commits to the first variant which parses a tag
    // with tuple enums, this can lead to variants being impossible to deserialize
    // like `Case2` is here.
    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum TupleEnum {
        Case1(bool, u32),
        Case2(bool, bool),
    }

    let res: Result<TupleEnum, _> = serde_json::from_value(json!([true, true]));
    let err = res.unwrap_err();
    assert!(
        matches!(
            &*err.to_string(),
            r#"TupleEnum::Case1: invalid type: boolean `true`, expected u32"#
        ),
        "{err}",
    );
}

#[test]
fn fallthrough_basic() {
    #[allow(dead_code)]
    #[derive(serde_implicit_proc::Deserialize, Debug)]
    enum EnumWithFallThrough<T> {
        Multiple {
            #[serde_implicit(tag)]
            variants: Vec<u32>,
        },
        Single {
            one: T,
        },
    }

    // #[derive(serde::Deserialize)]
    // struct Other {
    //     field: u32,
    // }

    let res: Result<EnumWithFallThrough<u32>, _> = serde_json::from_value(json!(42));
    res.unwrap();

    let res: Result<EnumWithFallThrough<u32>, _> = serde_json::from_value(json!(42.5));
    let err = res.unwrap_err();

    assert!(
        matches!(
            &*err.to_string(),
            // todo provide more specific diagnostics
            r#"invalid type: floating point `42.5`, expected EnumWithFallThrough"#
        ),
        "{err}",
    );

    let res: Result<EnumWithFallThrough<u32>, _> =
        serde_json::from_value(json!({"variants": [32]}));
    res.unwrap();
}

#[test]
fn tuple_custom_tag_position_middle() {
    // Test tag at position 1 (middle position)
    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum TupleEnum {
        Case1(bool, #[serde_implicit(tag)] String, u32),
        Case2(u32, #[serde_implicit(tag)] bool),
    }

    // Case1: [false, "hello", 42] with tag "hello" at position 1
    let res: Result<TupleEnum, _> = serde_json::from_value(json!([false, "hello", 42]));
    assert!(res.is_ok());
    assert_eq!(
        res.unwrap(),
        TupleEnum::Case1(false, "hello".to_string(), 42)
    );

    // Case2: [99, true] with tag true at position 1
    let res: Result<TupleEnum, _> = serde_json::from_value(json!([99, true]));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), TupleEnum::Case2(99, true));
}

#[test]
fn tuple_custom_tag_position_last() {
    // Test tag at last position
    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum TupleEnum {
        Case1(u32, bool, #[serde_implicit(tag)] String),
        Case2(#[serde_implicit(tag)] u64),
    }

    // Case1: [42, true, "tag"] with tag "tag" at position 2 (last)
    let res: Result<TupleEnum, _> = serde_json::from_value(json!([42, true, "tag"]));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), TupleEnum::Case1(42, true, "tag".to_string()));

    // Case2: [999] with tag 999 at position 0
    let res: Result<TupleEnum, _> = serde_json::from_value(json!([999]));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), TupleEnum::Case2(999));
}

#[test]
fn tuple_mixed_tag_positions() {
    // Test different tag positions across variants
    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum MixedEnum {
        // Tag at position 0 (default, no attribute)
        First(String, u32),
        // Tag at position 1
        Second(bool, #[serde_implicit(tag)] u32, String),
        // Tag at position 2 (last)
        Third(u32, bool, #[serde_implicit(tag)] String),
    }

    // First variant: ["hello", 42] with tag "hello" at position 0
    let res: Result<MixedEnum, _> = serde_json::from_value(json!(["hello", 42]));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), MixedEnum::First("hello".to_string(), 42));

    // Second variant: [true, 123, "world"] with tag 123 at position 1
    let res: Result<MixedEnum, _> = serde_json::from_value(json!([true, 123, "world"]));
    assert!(res.is_ok());
    assert_eq!(
        res.unwrap(),
        MixedEnum::Second(true, 123, "world".to_string())
    );

    // Third variant: [99, false, "tag"] with tag "tag" at position 2
    let res: Result<MixedEnum, _> = serde_json::from_value(json!([99, false, "tag"]));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), MixedEnum::Third(99, false, "tag".to_string()));
}

#[test]
fn tuple_custom_tag_no_match() {
    #[derive(serde_implicit::Deserialize, Debug)]
    enum TupleEnum {
        Case1(#[serde_implicit(tag)] String, u32),
        Case2(bool, #[serde_implicit(tag)] u32),
    }

    // [42, "hello"] doesn't match Case1 (expects String at pos 0) or Case2 (expects bool at pos 0)
    let res: Result<TupleEnum, _> = serde_json::from_value(json!([42, "hello"]));
    assert!(res.is_err());
}

#[test]
fn tuple_custom_tag_overlapping_resolved() {
    // With custom tag positions, the overlapping issue from tuple_overlap test is resolved
    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum TupleEnum {
        Case1(bool, #[serde_implicit(tag)] u32),  // Tag at position 1
        Case2(#[serde_implicit(tag)] bool, bool), // Tag at position 0
    }

    // [true, true] should now match Case2 (tag at position 0)
    let res: Result<TupleEnum, _> = serde_json::from_value(json!([true, true]));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), TupleEnum::Case2(true, true));

    // [false, 42] should match Case1 (tag at position 1)
    let res: Result<TupleEnum, _> = serde_json::from_value(json!([false, 42]));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), TupleEnum::Case1(false, 42));
}

#[test]
fn tuple_commit_semantics_verification() {
    // Verify that tag match commits to the variant
    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum TupleEnum {
        Case1(bool, u32),
        Case2(bool, bool),
    }

    // [false, false] - tag matches Case1 (bool at pos 0), commits, then fails on u32
    // Should NOT fall through to Case2, even though Case2 would succeed
    let res: Result<TupleEnum, _> = serde_json::from_value(json!([false, false]));
    let err = res.unwrap_err();
    assert!(
        err.to_string().contains("expected u32") || err.to_string().contains("invalid type"),
        "Expected error about u32, got: {}",
        err
    );
}

#[test]
fn tuple_flatten_basic() {
    // Test basic flatten functionality
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct A(String, bool);

    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum Implicit {
        Normal(bool),
        Tagged(u64, #[serde_implicit(tag)] u32),
        Nested(#[serde_implicit(flatten)] A),
    }

    // Should match Normal variant first
    let res: Result<Implicit, _> = serde_json::from_value(json!([false]));
    assert_eq!(res.unwrap(), Implicit::Normal(false));

    // Should match Tagged variant (tag at position 1)
    let res: Result<Implicit, _> = serde_json::from_value(json!([42, 99]));
    assert_eq!(res.unwrap(), Implicit::Tagged(42, 99));

    // Should match Nested (flatten) variant as fallback
    let res: Result<Implicit, _> = serde_json::from_value(json!(["hello", true]));
    assert_eq!(res.unwrap(), Implicit::Nested(A("hello".to_string(), true)));
}

#[test]
fn tuple_flatten_multiple() {
    // Test multiple flatten variants tried sequentially
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct StringBool(String, bool);

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct StringU64(String, u64);

    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum Multi {
        Normal(bool),
        Flatten1(#[serde_implicit(flatten)] StringBool),
        Flatten2(#[serde_implicit(flatten)] StringU64),
    }

    // Should match Normal variant
    let res: Result<Multi, _> = serde_json::from_value(json!([true]));
    assert_eq!(res.unwrap(), Multi::Normal(true));

    // Should match Flatten1 variant (String, bool)
    let res: Result<Multi, _> = serde_json::from_value(json!(["test", false]));
    assert_eq!(
        res.unwrap(),
        Multi::Flatten1(StringBool("test".to_string(), false))
    );

    // Should match Flatten2 variant (String, u64)
    let res: Result<Multi, _> = serde_json::from_value(json!(["test", 42]));
    assert_eq!(
        res.unwrap(),
        Multi::Flatten2(StringU64("test".to_string(), 42))
    );
}

#[test]
fn tuple_flatten_fallback_only() {
    // Test that flatten variants are only tried after all regular variants fail
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct Fallback(String, String);

    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum TestEnum {
        First(bool),
        Second(u64),
        Fall(#[serde_implicit(flatten)] Fallback),
    }

    // Regular variants should be tried first
    let res: Result<TestEnum, _> = serde_json::from_value(json!([true]));
    assert_eq!(res.unwrap(), TestEnum::First(true));

    let res: Result<TestEnum, _> = serde_json::from_value(json!([123]));
    assert_eq!(res.unwrap(), TestEnum::Second(123));

    // Flatten variant is only tried when others fail
    let res: Result<TestEnum, _> = serde_json::from_value(json!(["foo", "bar"]));
    assert_eq!(
        res.unwrap(),
        TestEnum::Fall(Fallback("foo".to_string(), "bar".to_string()))
    );
}

#[test]
fn tuple_flatten_no_match() {
    // Test error when no variant matches
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct OnlyBools(bool, bool);

    #[derive(serde_implicit::Deserialize, Debug, PartialEq)]
    enum TestEnum {
        First(u64),
        Second(#[serde_implicit(flatten)] OnlyBools),
    }

    // Should fail because it's not a u64 and not two bools
    let res: Result<TestEnum, _> = serde_json::from_value(json!(["string", "string"]));
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.to_string().contains("data did not match any variant"));
}

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

#[test]
fn test_string_key_map_to_integer_key() {
    use std::collections::HashMap;

    #[derive(serde_implicit_proc::Deserialize, Debug)]
    enum WithMap {
        Variant {
            #[serde_implicit(tag)]
            tag: String,
            data: HashMap<u32, String>,
        },
    }

    let res: Result<WithMap, _> = serde_json::from_value(
        json!({"tag": "hello", "data": {"0": "zero", "1": "one"}}),
    );
    let val = res.unwrap();
    match val {
        WithMap::Variant { data, .. } => {
            assert_eq!(data.len(), 2);
            assert_eq!(data[&0], "zero");
            assert_eq!(data[&1], "one");
        }
    }
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

#[test]
fn test_readme_tuples() {
    #[derive(serde_implicit::Deserialize, Debug)]
    enum Message {
        Literal(u64),
        BigOp(Op, Vec<Message>),
    }

    #[derive(serde::Deserialize, Debug)]
    enum Op {
        Sum,
    }

    let res: Result<Op, _> = serde_json::from_value(json!("Sum"));
    res.unwrap();

    let res: Result<Message, _> = serde_json::from_value(json!(["Sum", 1]));
    assert!(res.is_err());
    let err = res.unwrap_err();
    println!("{err:?}");
    assert!(
        err.to_string()
            .contains("Message::BigOp: invalid type: integer `1`, expected a sequence")
    );
}
