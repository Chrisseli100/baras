/// An alacrity-increasing buff effect, generated from `data/alacrity_abilities.csv`.
#[derive(Debug)]
pub struct AlacrityBuff {
    /// Alacrity increase as a decimal (0.1 = 10%).
    /// Multiplied by stack count when `is_stack` is true.
    pub amount: f32,
    pub is_stack: bool,
    /// Base duration in seconds — safety-net timeout for missed remove events.
    pub duration_secs: f32,
}

include!(concat!(env!("OUT_DIR"), "/alacrity_buffs.rs"));
