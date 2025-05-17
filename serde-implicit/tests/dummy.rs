use serde_implicit_proc::Deserialize;
use serde_json::json;

#[test]
fn test_basic() {
    #[derive(Deserialize, Debug)]
    enum Omg {
        Var1 {
            #[tag]
            unique_key: bool,
            other_key: u32,
        },
        Var2 {
            #[tag]
            blob: u32,
            other_key: u32,
        },
    }

    #[derive(serde::Deserialize, Debug)]
    #[serde(untagged)]
    enum Omg2 {
        Var1 { unique_key: bool, other_key: u32 },
        Var2 { blob: u32, other_key: u32 },
    }

    let res: Result<Omg, _> = serde_json::from_value(json!({"unique_key": true, "other_key": 09 }));
    assert!(res.is_ok());

    let res: Result<Omg, _> = serde_json::from_value(json!({"blob": 123, "other_key": 09 }));
    println!("{res:?}");
    assert!(res.is_ok());

    let res: Result<Omg, _> =
        serde_json::from_value(json!({"unique_key": true, "missing_key": true }));
    println!("{res:?}");

    let res: Result<Omg2, _> =
        serde_json::from_value(json!({"unique_key": true, "missing_key": true }));
    println!("{res:?}");

    assert!(res.is_ok());
}
