use crate::components::clear_core_motor::ClearCoreMotor;
use crate::components::scale::Scale;
use tokio::time::{Duration, Instant};

pub struct Node<'a> {
    pub scale: &'a Scale,
    pub motor: ClearCoreMotor, // Maybe this should be a reference?
}

impl<'a> Node <'a> {
    pub fn new(scale: &'a Scale, motor: ClearCoreMotor) -> Self {
        Self {scale, motor }
    }

    pub fn connect(mut scale: Scale) -> Scale {
        scale.connect().expect("Scale failed to connect");
        scale
    }

    pub async fn dispense(&self,
                          serving: f64,
                          sample_rate: f64,
                          cutoff_frequency: f64,
                          motor_speed: isize,
    ) {
        // Set LP filter values
        let filter_period = 1. / sample_rate;
        let filter_rc = 1. / (cutoff_frequency * 2. * std::f64::consts::PI);
        let filter_a = filter_period / (filter_period + filter_rc);
        let filter_b = filter_rc / (filter_period + filter_rc);

        // Initialize dispense tracking variables
        let init_time = Instant::now();
        let mut last_sent_motor = Instant::now();
        let init_weight = self.scale.weight_by_median(500, 100)
            .expect("Failed to weigh scale");
        let mut curr_weight = init_weight;
        let target_weight = init_weight - serving;
        let timeout = Duration::from_secs(90);
        let send_command_delay = Duration::from_millis(250);

        while curr_weight < target_weight {
            let curr_time = Instant::now();
            if curr_time - init_time > timeout {
                println!("WARNING: Dispense timed out!");
                break;
            }
            curr_weight = filter_a * self.scale.live_weigh().expect("Failed to weigh scale") + filter_b * curr_weight;

            if curr_time - last_sent_motor > send_command_delay {
                last_sent_motor = Instant::now();
                let err = (curr_weight - target_weight) / serving;
                let mut new_motor_speed = err * (motor_speed as f64);
                self.motor.set_velocity(new_motor_speed as isize).await.expect("Failed to change speed");
                self.motor.relative_move(1000).await.expect("Failed to update");
            }
        }

        let final_weight = self.scale.weight_by_median(500, 100).expect("Failed to weigh scale");
        println!("Dispensed: {:.1} g", final_weight);
    }

    pub async fn mock_dispense(&self,
                               dispense_time: Duration,
                               sample_rate: f64,
                               cutoff_frequency: f64,
                               motor_speed: isize,
    ) -> (Vec<Duration>, Vec<f64>) {
        // Set LP filter values
        let filter_period = 1. / sample_rate;
        let filter_rc = 1. / (cutoff_frequency * 2. * std::f64::consts::PI);
        let filter_a = filter_period / (filter_period + filter_rc);
        let filter_b = filter_rc / (filter_period + filter_rc);

        // Initialize dispense tracking variables
        let init_time = Instant::now();
        let mut last_sent_motor = Instant::now();
        let init_weight = self.scale.weight_by_median(500, 100)
            .expect("Failed to weigh scale");
        let mut curr_weight = init_weight;
        let timeout = dispense_time;
        let send_command_delay = Duration::from_millis(250);

        // Data tracking
        let mut times = Vec::new();
        let mut weights = Vec::new();

        loop {
            let curr_time = Instant::now();
            if curr_time - init_time > timeout {
                break;
            }
            curr_weight = filter_a * self.scale.live_weigh().expect("Failed to weigh scale") + filter_b * curr_weight;

            times.push(curr_time-init_time);
            weights.push(curr_weight);

            if curr_time - last_sent_motor > send_command_delay {
                last_sent_motor = Instant::now();
                self.motor.relative_move(1000).await.expect("Failed to update");
            }
        }

        let final_weight = self.scale.weight_by_median(500, 100).expect("Failed to weigh scale");
        println!("Dispensed: {:.1} g", final_weight);
        (times, weights)
    }
}



// enum NodeError {
//     ScaleError(ScaleError),
//     MotorError
// }

// #[tokio::test]
// async fn dispense_test() -> Data {
//     let mut scale = Scale::new(716620).expect("Failed to construct scale");
//     scale.connect().expect("Failed to connect scale");
//     let (tx, rx) = mpsc::channel::<Message>(100);
//     let client = tokio::spawn(client("192.168.1.12:8888", rx));
//     let motor = AsyncMotor::new(0, 800, Controller::new(tx));
//     
//     let (_, _, data) = Node::dispense(scale, motor, 75., 50, 200., 0.5).await;
//     data
// }
