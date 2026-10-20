#![cfg(feature = "serde")]

use finance_rs::Date;

#[test]
fn deserialization_rejects_unrecognized_fields() {
    let input = r#"{"year":2024,"month":1,"day":2,"unexpected":true}"#;
    let parsed: Result<Date, _> = serde_json::from_str(input);
    assert!(parsed.is_err());
}
