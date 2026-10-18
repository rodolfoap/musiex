//! A small deterministic random number generator.
//!
//! It is kept in-tree (PCG32) instead of using the `rand` crate so that a given
//! seed produces exactly the same exercise on every platform and across
//! dependency upgrades. Shared links and golden tests rely on that.

#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
    const STREAM: u64 = 0xda3e_39cb_94b9_5bdb;

    pub fn new(seed: u64) -> Self {
        let mut rng = Rng {
            state: 0,
            inc: (Self::STREAM << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(Self::MULTIPLIER).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform integer in `0..n`. `n` must be greater than zero.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "Rng::below called with n = 0");
        // Lemire's multiply-and-reject method: unbiased and cheap.
        let threshold = n.wrapping_neg() % n;
        loop {
            let m = u64::from(self.next_u32()) * u64::from(n);
            if (m as u32) >= threshold {
                return (m >> 32) as u32;
            }
        }
    }

    /// Uniform integer in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi, "Rng::range called with lo > hi");
        lo + self.below((hi - lo + 1) as u32) as i32
    }

    /// True with probability `p` (clamped to `0.0..=1.0`).
    pub fn chance(&mut self, p: f64) -> bool {
        f64::from(self.next_u32()) < p.clamp(0.0, 1.0) * 4_294_967_296.0
    }

    /// Picks an index according to integer weights. Returns `None` if all weights are zero.
    pub fn weighted(&mut self, weights: &[u32]) -> Option<usize> {
        let total: u32 = weights.iter().sum();
        if total == 0 {
            return None;
        }
        let mut roll = self.below(total);
        for (i, &w) in weights.iter().enumerate() {
            if roll < w {
                return Some(i);
            }
            roll -= w;
        }
        unreachable!("roll is always below the total weight")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        let same = (0..32).filter(|_| a.next_u32() == b.next_u32()).count();
        assert!(same < 2);
    }

    #[test]
    fn stream_is_stable() {
        // Guards against accidental changes to the algorithm: seeds must keep
        // producing the same exercises.
        let mut rng = Rng::new(0);
        let first: Vec<u32> = (0..3).map(|_| rng.next_u32()).collect();
        assert_eq!(first, [174_444_157, 2_543_941_438, 3_225_358_906]);
    }

    #[test]
    fn range_is_inclusive_and_bounded() {
        let mut rng = Rng::new(7);
        let mut seen = [false; 5];
        for _ in 0..1000 {
            let v = rng.range(-2, 2);
            assert!((-2..=2).contains(&v));
            seen[(v + 2) as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }
}
