use super::*;

#[test]
fn input_price_is_per_million_tokens() {
    let p = input_per_mtok_usd("claude-sonnet-4-6").expect("catalogued model");
    assert!((p - 3.0).abs() < 1e-6, "got {p}");
}

#[test]
fn unpriced_model_has_no_price() {
    assert_eq!(input_per_mtok_usd("no-such-model-xyz"), None);
}

#[test]
fn no_request_when_not_smaller() {
    assert!(request(CompressorKind::SmartCrusher, 100, 100).is_none());
    assert!(request(CompressorKind::SmartCrusher, 50, 100).is_none());
}

#[test]
fn request_carries_attribution_and_token_counts() {
    let req = request(CompressorKind::SmartCrusher, 2000, 1000).unwrap();
    assert_eq!(req.model, attribution_model());
    assert_eq!(req.compressor, CompressorKind::SmartCrusher.as_str());
    assert_eq!((req.original_tokens, req.compacted_tokens), (2000, 1000));
}
