use proptest::prelude::*;
use proptest::proptest;
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
}
