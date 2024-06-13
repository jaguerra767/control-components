use crate::components::clear_core_motor::ClearCoreMotor;
use crate::components::scale::Scale;
use tokio::time::{Duration, Instant};

pub struct Node<'a> {
    scale: &'a Scale,
    motor: ClearCoreMotor, // Maybe this should be a reference? 
}

impl<'a> Node <'a> {
    pub fn new(scale: &'a Scale, motor: ClearCoreMotor) -> Self {
        Self {scale, motor }
    }

    pub fn connect(mut scale: Scale) -> Scale {
        scale.connect().expect("Scale failed to connect");
        scale
    }

    pub async fn dispense(&self, serving: f64, sample_rate: f64, cutoff_frequency: f64) {
        // Instantiate motor handles
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
                //TODO: Add code that changes motor speed depending on weight error
                self.motor.relative_move(1000).await.expect("Failed to update");
            }
        }
    }
}

    //TODO: We can figure out how to do this later
// #[derive(Serialize, Deserialize)]
// pub struct Data {
//     pub time: Vec<Duration>,
//     pub weight: Vec<f64>,
// }
// 
// impl Data {
//     pub fn new(init_time: Duration, init_weight: f64) -> Self {
//         Self {
//             time: vec![init_time],
//             weight: vec![init_weight]
//         }
//     }
//     pub fn log(&mut self,
//                time: Duration,
//                weight: f64
//     ) {
//         self.time.push(time);
//         self.weight.push(weight);
//     }
//     
//     pub fn plot(&self) {
//         // Create a sample Data struct
//         let data = Data {
//             time: vec![Duration::from_millis(500), Duration::from_millis(500*2), Duration::from_millis(500*3)],
//             weight: vec![10.5, 20.2, 30.7],
//         };
// 
//         // Create the "data" directory if it doesn't exist
//         let data_dir = Path::new("data");
//         if !data_dir.exists() {
//             create_dir_all(data_dir).expect("Failed to create data directory");
//         }
// 
//         // Open a file for writing in the "data" directory
//         let mut file = File::create(data_dir.join("data.json")).expect("Failed to create file");
// 
//         // Write the data to the JSON file
//         serde_json::to_writer_pretty(&mut file, &data).expect("Failed to write to file");
//     }
// }
// impl std::process::Termination for Data {
//     fn report(self) -> std::process::ExitCode {
//         // Implement the report method based on the semantics of your Data type
//         // and return an appropriate ExitStatus value
//         unimplemented!()
//     }
// }
// 



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
