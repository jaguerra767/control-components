use crate::components::clear_core_io::{AnalogInput, DigitalOutput, HBridge, HBridgeState};
use crate::controllers::clear_core::Error;
pub use crate::controllers::clear_core::Message;
use crate::controllers::ek1100_io::IOCard;


pub struct SimpleLinearActuator {
    output: HBridge,
    feedback: Option<AnalogInput>,
}

impl SimpleLinearActuator {
    pub fn new(output: HBridge) -> Self {
        Self {
            output,
            feedback: None,
        }
    }

    pub fn with_feedback(output: HBridge, feedback: AnalogInput) -> Self {
        Self {
            output,
            feedback: Some(feedback),
        }
    }
    pub async fn get_feedback(&self) -> Result<Option<isize>, Error> {
        if let Some(fb) = self.feedback.as_ref() {
            Ok(Some(fb.get_state().await?))
        } else {
            Ok(None)
        }
    }
    pub async fn actuate(&self, state: HBridgeState) -> Result<(), Error> {
        self.output.set_state(state).await
    }
}

#[derive(Clone, Copy)]
pub enum ActuatorCh {
    Cha,
    Chb,
}

#[derive(Debug)]
pub enum Output {
    EtherCat(IOCard, usize, u8),
    ClearCore(DigitalOutput),
}

impl Output {
    pub async fn set_state(&mut self, state: bool) -> Result<(), Error> {
        match self {
            Output::EtherCat(io, slot, id) => {
                io.set_state(*slot, *id, state).await;
                Ok(())
            }
            Output::ClearCore(out) => {
                out.set_state(state).await
            }
        }
    }
}

#[derive(Debug)]
pub struct RelayHBridge {
    fb_pair: (AnalogInput, Option<AnalogInput>),
    output_pair: (Output, Output),
}

impl RelayHBridge {
    pub fn new(outputs: (Output, Output), feedback: AnalogInput) -> Self {
        Self {
            fb_pair: (feedback, None),
            output_pair: outputs,
        }
    }

    pub fn with_dual_fb(outputs: (Output, Output), feedback: (AnalogInput, AnalogInput)) -> Self {
        Self {
            fb_pair: (feedback.0, Some(feedback.1)),
            output_pair: outputs,
        }
    }

    pub async fn get_feedback(&self) -> Result<isize, Error> {
        let mut position = self.fb_pair.0.get_state().await?;
        if let Some(fb) = &self.fb_pair.1 {
            let pos_b = fb.get_state().await?;
            position = (position + pos_b) / 2
        }
       Ok(position) 
    }

    pub async fn actuate(&mut self, power: HBridgeState) -> Result<(), Error> {
        match power {
            HBridgeState::Pos => {
                self.output_pair.0.set_state(true).await
            }
            HBridgeState::Neg => {
                self.output_pair.1.set_state(true).await
            }
            HBridgeState::Off => {
                self.output_pair.0.set_state(false).await?;
                self.output_pair.1.set_state(false).await
            }
        }
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
