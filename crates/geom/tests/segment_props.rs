//! Property tests: segment distance and intersection.

use proptest::prelude::*;
use protasena_geom::{Intersection, Pt, Segment};

fn coord() -> impl Strategy<Value = f64> {
    (-1000i32..=1000).prop_map(f64::from)
}

fn pt() -> impl Strategy<Value = Pt> {
    (coord(), coord()).prop_map(|(x, y)| Pt::new(x, y))
}

fn segment() -> impl Strategy<Value = Segment> {
    (pt(), pt()).prop_map(|(a, b)| Segment::new(a, b))
}

/// Within floating-point slack of the coordinate range used here.
fn near(d: f64) -> bool {
    d <= 1e-9
}

proptest! {
    #[test]
    fn closest_point_lies_on_segment_and_is_nearest(s in segment(), p in pt(), t in 0.0f64..=1.0) {
        let c = s.closest_point(p);
        prop_assert!(near(s.distance_to_pt(c)));
        let other = s.a.lerp(s.b, t);
        prop_assert!(p.distance(c) <= p.distance(other) + 1e-9);
    }

    #[test]
    fn intersection_point_lies_on_both(a in segment(), b in segment()) {
        match a.intersect(b) {
            Intersection::Disjoint => {}
            Intersection::Point(p) => {
                prop_assert!(near(a.distance_to_pt(p)));
                prop_assert!(near(b.distance_to_pt(p)));
            }
            Intersection::Overlap(o) => {
                for p in [o.a, o.b, o.a.midpoint(o.b)] {
                    prop_assert!(near(a.distance_to_pt(p)));
                    prop_assert!(near(b.distance_to_pt(p)));
                }
            }
        }
    }

    #[test]
    fn intersection_is_symmetric(a in segment(), b in segment()) {
        match (a.intersect(b), b.intersect(a)) {
            (Intersection::Disjoint, Intersection::Disjoint) => {}
            (Intersection::Point(p), Intersection::Point(q)) => prop_assert!(near(p.distance(q))),
            (Intersection::Overlap(o), Intersection::Overlap(q)) => {
                prop_assert!(near(o.length() - q.length()) && near(q.length() - o.length()));
            }
            (x, y) => prop_assert!(false, "asymmetric: {:?} vs {:?}", x, y),
        }
    }

    #[test]
    fn crossing_detected_when_endpoints_straddle(a in segment(), b in segment()) {
        // Strict straddling in both directions means a proper crossing.
        let side = |s: Segment, p: Pt| s.direction().cross(p - s.a);
        let straddles = |s: Segment, t: Segment| side(s, t.a) * side(s, t.b) < 0.0;
        if straddles(a, b) && straddles(b, a) {
            prop_assert!(matches!(a.intersect(b), Intersection::Point(_)));
        }
    }

    #[test]
    fn hostile_segments_never_panic(c in proptest::array::uniform8(any::<f64>())) {
        let a = Segment::new(Pt::new(c[0], c[1]), Pt::new(c[2], c[3]));
        let b = Segment::new(Pt::new(c[4], c[5]), Pt::new(c[6], c[7]));
        let _ = a.intersect(b);
        let _ = a.distance_to_pt(b.a);
    }
}
