use super::*;

#[test]
fn empty_stream_reports_empty() {
    assert!(CashFlowStream::default().is_empty());
    assert!(!CashFlowStream::new([CashFlow::new(
        Date::new(2024, 1, 1).unwrap(),
        0.0,
        FlowKind::Other,
    )])
    .unwrap()
    .is_empty());
}
