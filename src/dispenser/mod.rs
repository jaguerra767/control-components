pub mod setpoint_dispense;
pub mod timed_dispense;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Parameters {
    motor_speed: f64,
    sample_rate: f64,
    cutoff_frequency: f64,
    check_offset: f64,
    stop_offset: f64,
}

impl Parameters {
    pub fn new(
        motor_speed: f64,
        sample_rate: f64,
        cutoff_frequency: f64,
        check_offset: f64,
        stop_offset: f64,
    ) -> Self {
        Self {
            motor_speed,
            sample_rate,
            cutoff_frequency,
            check_offset,
            stop_offset,
        }
    }
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            motor_speed: 0.3,
            sample_rate: 50.0,
            cutoff_frequency: 0.5,
            check_offset: 15.0,
            stop_offset: 7.0,
        }
    }
}
