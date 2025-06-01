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
            r#"invalid type: boolean `true`, expected u32"#
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
