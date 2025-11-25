use arbitrary_json::ArbitraryValue;
use proptest::prelude::*;
use proptest::proptest;
use proptest_arbitrary_interop::arb;
use proptest_derive::Arbitrary;

#[derive(serde_implicit_proc::Deserialize, serde::Serialize, Debug, PartialEq, Arbitrary)]
#[serde(untagged)]
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
    },
    BoolVariant {
        #[serde_implicit(tag)]
        bool_tag: bool,
        value: Vec<String>,
    },
}

#[derive(serde_implicit_proc::Deserialize, serde::Serialize, Debug, PartialEq, Arbitrary)]
#[serde(untagged)]
enum OverlappingFields {
    Variant1 {
        #[serde_implicit(tag)]
        type_tag: String,
        common_field: u32,
        variant1_specific: bool,
    },
    Variant2 {
        #[serde_implicit(tag)]
        version: u32,
        common_field: u32,
        variant2_specific: String,
    },
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Arbitrary)]
struct NestedData {
    field1: String,
    field2: u32,
}

#[derive(serde_implicit_proc::Deserialize, serde::Serialize, Debug, PartialEq, Arbitrary)]
#[serde(untagged)]
enum NestedEnum {
    Simple {
        #[serde_implicit(tag)]
        tag: String,
        value: u32,
    },
    Complex {
        #[serde_implicit(tag)]
        complex_tag: bool,
        nested: NestedData,
        optional: Option<String>,
    },
}

#[derive(serde_implicit_proc::Deserialize, serde::Serialize, Debug, PartialEq)]
enum RecursiveEnum {
    Leaf {
        #[serde_implicit(tag)]
        is_leaf: bool,
        value: String,
    },
    Node {
        #[serde_implicit(tag)]
        has_children: bool,
        children: Vec<RecursiveEnum>,
        metadata: String,
    },
}

mod edge_cases {
    #[derive(serde_implicit_proc::Deserialize, serde::Serialize, Debug, PartialEq)]
    enum EmptyEnum {}

    #[derive(serde_implicit_proc::Deserialize, serde::Serialize, Debug, PartialEq)]
    enum SingleVariant {
        OnlyVariant {
            #[serde_implicit(tag)]
            this_is_it: bool,
            data: String,
        },
    }
}

/// Basic tuple enum - discriminated by first field type
#[derive(serde_implicit::Deserialize, serde::Serialize, Debug, PartialEq, Arbitrary)]
#[serde(untagged)]
enum TupleEnum {
    BoolU32(bool, u32),
    StringOnly(String),
    U64Vec(u64, Vec<u32>),
}

/// Tuple enum with custom tag positions via #[serde_implicit(tag)]
#[derive(serde_implicit::Deserialize, serde::Serialize, Debug, PartialEq, Arbitrary)]
#[serde(untagged)]
enum TupleCustomTag {
    /// Tag at position 1
    MiddleTag(bool, #[serde_implicit(tag)] String, u32),
    /// Tag at position 0 (explicit)
    FirstTag(#[serde_implicit(tag)] u64, bool),
    /// Tag at last position
    LastTag(u32, bool, #[serde_implicit(tag)] String),
}

/// Helper struct for flatten tests
#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Arbitrary)]
struct FlattenInner(String, bool);

/// Tuple enum with flatten for fallback variants
#[derive(serde_implicit::Deserialize, serde::Serialize, Debug, PartialEq, Arbitrary)]
#[serde(untagged)]
enum TupleFlatten {
    Normal(bool),
    Tagged(u64, #[serde_implicit(tag)] u32),
    Fallback(#[serde_implicit(flatten)] FlattenInner),
}

proptest! {
    #[test]
    fn test_tags_different_types(tag in any::<MultiTypeTag>()) {
        let serialized = serde_json::to_string(&tag).unwrap();
        let deserialized: MultiTypeTag = serde_json::from_str(&serialized).unwrap();
        assert_eq!(tag, deserialized);
    }

    #[test]
    fn test_tags_overlapping_fields(tag in any::<OverlappingFields>()) {
        let serialized = serde_json::to_string(&tag).unwrap();
        let deserialized = serde_json::from_str(&serialized).unwrap();
        assert_eq!(tag, deserialized);
    }

    #[test]
    fn test_tags_nested_enum(tag in any::<NestedEnum>()) {
        let serialized = serde_json::to_string(&tag).unwrap();
        let deserialized = serde_json::from_str(&serialized).unwrap();
        assert_eq!(tag, deserialized);
    }

    // Verifies that serde-implicit and serde(untagged) parse the same types
    // the only difference should be in their behavior in error messages
    #[test]
    fn test_agrees_with_serde(rand in any::<MultiTypeTag>()) {
        #[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
        #[serde(untagged)]
        enum SerdeMultiTypeTag {
            StringVariant {
                string_tag: String,
                value: u32,
            },
            NumberVariant {
                number_tag: u64,
                value: String,
            },
            BoolVariant {
                bool_tag: bool,
                value: Vec<String>,
            },
        }

        let serialized = serde_json::to_string(&rand).unwrap();

        let deserialized1  : MultiTypeTag = serde_json::from_str(&serialized).unwrap();
        let deserialized2  : SerdeMultiTypeTag = serde_json::from_str(&serialized).unwrap();

        assert_eq!(serde_json::to_string(&deserialized1).unwrap(), serde_json::to_string(&deserialized2).unwrap());
    }

    /// Fuzz test: arbitrary JSON should never cause a panic during deserialization.
    /// We don't care if it returns Ok or Err, just that it terminates cleanly.
    #[test]
    fn fuzz_multi_type_tag_no_panic(json in arb::<ArbitraryValue>()) {
        let _ = serde_json::from_value::<MultiTypeTag>(json.into());
    }

    #[test]
    fn fuzz_overlapping_fields_no_panic(json in arb::<ArbitraryValue>()) {
        let _ = serde_json::from_value::<OverlappingFields>(json.into());
    }

    #[test]
    fn fuzz_nested_enum_no_panic(json in arb::<ArbitraryValue>()) {
        let _ = serde_json::from_value::<NestedEnum>(json.into());
    }

    #[test]
    fn fuzz_tuple_enum_no_panic(json in arb::<ArbitraryValue>()) {
        let _ = serde_json::from_value::<TupleEnum>(json.into());
    }

    #[test]
    fn fuzz_tuple_custom_tag_no_panic(json in arb::<ArbitraryValue>()) {
        let _ = serde_json::from_value::<TupleCustomTag>(json.into());
    }

    #[test]
    fn fuzz_tuple_flatten_no_panic(json in arb::<ArbitraryValue>()) {
        let _ = serde_json::from_value::<TupleFlatten>(json.into());
    }

    #[test]
    fn test_tuple_enum_roundtrip(value in any::<TupleEnum>()) {
        let serialized = serde_json::to_value(&value).unwrap();
        let deserialized: TupleEnum = serde_json::from_value(serialized).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    fn test_tuple_custom_tag_roundtrip(value in any::<TupleCustomTag>()) {
        let serialized = serde_json::to_value(&value).unwrap();
        let deserialized: TupleCustomTag = serde_json::from_value(serialized).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    fn test_tuple_flatten_roundtrip(value in any::<TupleFlatten>()) {
        let serialized = serde_json::to_value(&value).unwrap();
        let deserialized: TupleFlatten = serde_json::from_value(serialized).unwrap();
        assert_eq!(value, deserialized);
    }
}
