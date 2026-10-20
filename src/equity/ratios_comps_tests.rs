use super::*;

#[test]
fn peer_multiple_stats_cover_all_aggregations_and_validation() {
    let median = comps_implied_value(10.0, &[1.0, 2.0, 5.0, 8.0], PeerStat::Median).unwrap();
    let mean = comps_implied_value(10.0, &[1.0, 2.0, 5.0, 8.0], PeerStat::Mean).unwrap();
    let quartiles = comps_implied_value(10.0, &[1.0, 2.0, 5.0, 8.0], PeerStat::Quartiles).unwrap();
    assert_eq!(median.low, median.central);
    assert_eq!(mean.central, 40.0);
    assert!(quartiles.low < quartiles.central && quartiles.central < quartiles.high);
    assert!(comps_implied_value(1.0, &[], PeerStat::Median).is_err());
    assert!(comps_implied_value(1.0, &[-1.0], PeerStat::Median).is_err());
    assert!(comps_implied_value(f64::NAN, &[1.0], PeerStat::Median).is_err());
}
