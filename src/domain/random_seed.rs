use rand::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomSeed(u64);

impl RandomSeed {
    pub fn from_u64(seed: u64) -> Self {
        Self(seed)
    }

    pub fn new() -> Self {
        let mut rng = rand::rngs::OsRng;
        Self(rng.next_u64())
    }

    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn split(&self) -> (RandomSeed, RandomSeed) {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(self.0);
        let seed1 = RandomSeed::from_u64(rng.next_u64());
        let seed2 = RandomSeed::from_u64(rng.next_u64());
        (seed1, seed2)
    }
}
