use crate::components::clear_core_io::{AnalogInput, HBridge, HBridgeState, Output, OutputState};
pub use crate::controllers::clear_core::Message;
use std::error::Error;
use std::future::Future;
use tokio::sync::mpsc::Sender;

//TODO: Move this to a hatches module
#[allow(unused)]
const ACTUONIX_LA_MAX_STROKE: isize = 34000;
//TODO: Move this to a hatches module
#[allow(unused)]
const ACTUONIX_LA_MIN_STROKE: isize = 400;
//TODO: Move this to a hatches module
#[allow(unused)]

pub trait LinearActuator {
    fn get_feedback(&self) -> impl Future<Output = Result<isize, Box<dyn Error>>> + Send;
    fn actuate(
        &self,
        power: HBridgeState,
    ) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
}

pub struct SimpleLinearActuator {
    output: HBridge,
    feedback: AnalogInput,
}

impl LinearActuator for SimpleLinearActuator {
    async fn get_feedback(&self) -> Result<isize, Box<dyn Error>> {
        self.feedback.get_state().await
    }

    async fn actuate(&self, state: HBridgeState) -> Result<(), Box<dyn Error>> {
        self.output.set_state(state).await
    }
}

impl SimpleLinearActuator {
    pub fn new(sender: Sender<Message>, output_id: u8, feedback_id: u8) -> Self {
        Self {
            output: HBridge::new(output_id, 32000, sender.clone()),
            feedback: AnalogInput::new(feedback_id, sender),
        }
    }

    pub fn from_io(output: HBridge, feedback: AnalogInput) -> Self {
        Self { output, feedback }
    }
}

#[derive(Clone, Copy)]
pub enum ActuatorCh {
    Cha,
    Chb,
}

pub struct RelayHBridge {
    fb_pair: (AnalogInput, Option<AnalogInput>),
    output_pair: (Output, Output),
}

impl RelayHBridge {
    pub fn new(sender: Sender<Message>, output_pair_ids: (u8, u8), feedback_id: u8) -> Self {
        Self {
            fb_pair: (AnalogInput::new(feedback_id, sender.clone()), None),
            output_pair: (
                Output::new(output_pair_ids.0, sender.clone()),
                Output::new(output_pair_ids.1, sender),
            ),
        }
    }

    pub fn with_dual_feedback(
        sender: Sender<Message>,
        feedback_ids: (u8, u8),
        output_ids: (u8, u8),
    ) -> Self {
        Self {
            fb_pair: (
                AnalogInput::new(feedback_ids.0, sender.clone()),
                Some(AnalogInput::new(feedback_ids.1, sender.clone())),
            ),
            output_pair: (
                Output::new(output_ids.0, sender.clone()),
                Output::new(output_ids.1, sender),
            ),
        }
    }

    pub fn from_io(output_pair: (Output, Output), feedback: AnalogInput) -> Self {
        Self {
            fb_pair: (feedback, None),
            output_pair,
        }
    }

    pub fn from_io_with_dual_feedback(
        output_pair: (Output, Output),
        feedback_pair: (AnalogInput, AnalogInput),
    ) -> Self {
        Self {
            fb_pair: (feedback_pair.0, Some(feedback_pair.1)),
            output_pair,
        }
    }
}

impl LinearActuator for RelayHBridge {
    async fn get_feedback(&self) -> Result<isize, Box<dyn Error>> {
        let mut position = self.fb_pair.0.get_state().await?;
        if let Some(fb) = &self.fb_pair.1 {
            let pos_b = fb.get_state().await?;
            position = (position + pos_b) / 2
        }
        Ok(position)
    }

    async fn actuate(&self, power: HBridgeState) -> Result<(), Box<dyn Error>> {
        match power {
            HBridgeState::Pos => {
                self.output_pair.0.set_state(OutputState::On).await?;
            }
            HBridgeState::Neg => {
                self.output_pair.1.set_state(OutputState::On).await?;
            }
            HBridgeState::Off => {
                self.output_pair.0.set_state(OutputState::Off).await?;
                self.output_pair.1.set_state(OutputState::Off).await?;
            }
        }
        Ok(())
    }
}

// #[tokio::test]
// async fn linear_actuator_feedback_test() {
//     let (tx, rx) = mpsc::channel::<Message>(10);
//
//     let la_task_read_pos = tokio::spawn( async move {
//         let actuator = SimpleLinearActuator::new(
//             HBridge::new(4,CLEAR_CORE_H_BRIDGE_MAX),
//             Input::new(3),
//             Controller::new(tx));
//         let pos = actuator.get_feedback().await.expect("Failed to read");
//         println!("Actuator position: {} .", pos);
//     });
//
//     let client = tokio::spawn(tcp_client::client("192.168.1.11:8888", rx));
//     let _ = la_task_read_pos.await;
//     let _ = client.await;
// }
//
// #[tokio::test]
// async fn la_negative_dir_test() {
//     let (tx, rx) = mpsc::channel::<Message>(10);
//     let la_task = tokio::spawn( async move {
//         let actuator = SimpleLinearActuator::new(
//             HBridge::new(5,CLEAR_CORE_H_BRIDGE_MAX),
//             Input::new(4),
//             Controller::new(tx)
//         );
//
//         let _ = actuator.actuate(HBridgeState::Pos).await;
//         tokio::time::sleep(Duration::from_secs(2)).await;
//         let _ = actuator.actuate(HBridgeState::Off).await;
//     });
//     let client = tokio::spawn(tcp_client::client("192.168.1.11:8888", rx));
//     let _ = la_task.await;
//     let _ = client.await;
// }
//
// #[tokio::test]
// async fn test_mplex_actuator() {
//     let (tx, rx) = mpsc::channel::<Message>(10);
//
//     let actuator_task =  tokio::spawn(async move {
//
//
//         let actuators_a_b = MPlexActuatorPair::new(
//             (Output::new(2), HBridge::new(4, CLEAR_CORE_H_BRIDGE_MAX)),
//             (Input::new(3), Input::new(4)),
//             Controller::new(tx.clone())
//         );
//
//         let fb_a  = actuators_a_b.get_feedback(ActuatorCh::Cha).await.unwrap();
//         println!("Actuator A, currently at {fb_a}.");
//
//         actuators_a_b.actuate(ActuatorCh::Cha, HBridgeState::Pos).await.unwrap();
//         tokio::time::sleep(Duration::from_secs(2)).await;
//
//         actuators_a_b.actuate(ActuatorCh::Cha, HBridgeState::Neg).await.unwrap();
//         tokio::time::sleep(Duration::from_secs(2)).await;
//
//         actuators_a_b.actuate(ActuatorCh::Cha, HBridgeState::Off).await.unwrap();
//         tokio::time::sleep(Duration::from_secs(2)).await;
//
//         actuators_a_b.actuate(ActuatorCh::Chb, HBridgeState::Pos).await.unwrap();
//         tokio::time::sleep(Duration::from_secs(2)).await;
//
//         actuators_a_b.actuate(ActuatorCh::Chb, HBridgeState::Neg).await.unwrap();
//         tokio::time::sleep(Duration::from_secs(2)).await;
//
//         actuators_a_b.actuate(ActuatorCh::Chb, HBridgeState::Off).await.unwrap();
//         tokio::time::sleep(Duration::from_secs(2)).await;
//
//     });
//     let client = tokio::spawn(tcp_client::client("192.168.1.11:8888", rx));
//     let _ = actuator_task.await.unwrap();
//     let _ = client.await.unwrap();
// }
//
// #[tokio::test]
// async fn test_relay_h_bridge() {
//     let (tx, rx) = mpsc::channel::<Message>(10);
//     let relay_h_bridge = RelayHBridge::new(
//         Input::new(4),
//         (Output::new(2), Output::new(3)),
//         Controller::new(tx)
//     );
//
//     let actuator_task = tokio::spawn(async move {
//         let feedback = relay_h_bridge.get_feedback().await.unwrap();
//         //tokio::time::sleep(Duration::from_secs(6)).await;
//         //If feedback is plugged in this will never be 0
//         assert_ne!(feedback, 0);
//         // relay_h_bridge.actuate(HBridgeState::Pos).await.unwrap();
//         // tokio::time::sleep(Duration::from_millis(700)).await;
//         // relay_h_bridge.actuate(HBridgeState::Off).await.unwrap();
//         // tokio::time::sleep(Duration::from_millis(2000)).await;
//         relay_h_bridge.actuate(HBridgeState::Neg).await.unwrap();
//         tokio::time::sleep(Duration::from_millis(800)).await;
//         relay_h_bridge.actuate(HBridgeState::Off).await.unwrap();
//     });
//
//     let client = tokio::spawn(tcp_client::client("192.168.1.11:8888", rx));
//     let _ = actuator_task.await.unwrap();
//     let _ = client.await.unwrap();
//
// }
