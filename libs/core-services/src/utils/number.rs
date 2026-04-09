use n0_future::time::Instant;

pub struct ExponentialGrowth {
    start_time: Instant,
    initial_value: i64,
    step: i64,
    growth_rate: f64,
    max: i64
}

impl ExponentialGrowth {
    pub fn new(step: i64, growth_rate: f64, initial_value: i64, max: i64) -> Self {
        Self {
            start_time: Instant::now(),
            initial_value,
            step,
            growth_rate,
            max
        }
    }

    pub fn next(&self) -> i64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let growth = (self.growth_rate * elapsed * 0.1).exp();
        let raw_value = self.initial_value as f64 * growth;

        // Round down to nearest multiple of step
        let stepped_value = ((raw_value / self.step as f64).floor() * self.step as f64) as i64;

        // Ensure the value is at least initial_value
        stepped_value.max(self.initial_value).min(self.max)
    }
}
