//! Option and realized-volatility identities over ranges of valid inputs.
use finance_rs::options::{self, OptionKind};
use proptest::prelude::*;

proptest! {
    #[test]
    fn european_prices_increase_with_volatility(spot in 10.0..500.0_f64, strike in 10.0..500.0_f64,
                                                  t in 0.05..5.0_f64, rate in -0.05..0.15_f64,
                                                  vol1 in 0.03..0.8_f64, gap in 0.001..0.5_f64) {
        let vol2=(vol1+gap).min(1.5);
        for kind in [OptionKind::Call,OptionKind::Put] {
            let a=options::bsm(kind,spot,strike,t,rate,0.01,vol1).unwrap();
            let b=options::bsm(kind,spot,strike,t,rate,0.01,vol2).unwrap();
            prop_assert!(b+1e-10>=a);
        }
    }

    #[test]
    fn log_volatility_estimators_are_price_scale_invariant(scale in 0.1..100.0_f64) {
        let (o,h,l,c)=(vec![100.,101.,99.5,103.],vec![102.,104.,104.,105.],
                       vec![98.,99.,97.,101.],vec![101.,100.5,103.,102.]);
        let scaled=|v:&[f64]| v.iter().map(|x|x*scale).collect::<Vec<_>>();
        let (os,hs,ls,cs)=(scaled(&o),scaled(&h),scaled(&l),scaled(&c));
        let a=[options::close_to_close(&c,252.).unwrap(),options::parkinson(&h,&l,252.).unwrap(),
               options::garman_klass(&o,&h,&l,&c,252.).unwrap(),options::rogers_satchell(&o,&h,&l,&c,252.).unwrap(),
               options::yang_zhang(&o,&h,&l,&c,252.).unwrap()];
        let b=[options::close_to_close(&cs,252.).unwrap(),options::parkinson(&hs,&ls,252.).unwrap(),
               options::garman_klass(&os,&hs,&ls,&cs,252.).unwrap(),options::rogers_satchell(&os,&hs,&ls,&cs,252.).unwrap(),
               options::yang_zhang(&os,&hs,&ls,&cs,252.).unwrap()];
        for (x,y) in a.into_iter().zip(b) { prop_assert!((x-y).abs()<1e-10); }
    }

    #[test]
    fn ewma_vol_scales_linearly(scale in 0.1..10.0_f64) {
        let xs=[0.01,-0.02,0.005,0.03,-0.01];
        let scaled=xs.map(|x|x*scale);
        let a=options::ewma(&xs,0.94,252.).unwrap();
        let b=options::ewma(&scaled,0.94,252.).unwrap();
        prop_assert!((b-scale*a).abs()<1e-12);
    }
}
