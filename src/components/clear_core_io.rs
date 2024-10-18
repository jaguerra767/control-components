use crate::components::send_recv::SendRecv;
use crate::controllers::clear_core::{check_reply, Error, Message, CR, STX};
use crate::util::utils::{ascii_to_int, int_to_byte, num_to_bytes};
use tokio::sync::mpsc::Sender;

pub const CLEAR_CORE_H_BRIDGE_MAX: i16 = 32760;
#[derive(Clone)]
pub struct DigitalInput {
    cmd: [u8; 4],
    drive_sender: Sender<Message>,
}

impl DigitalInput {
    pub fn new(id: u8, drive_sender: Sender<Message>) -> Self {
        let cmd = [STX, b'I', int_to_byte(id), CR];
        Self { cmd, drive_sender }
    }

    pub async fn get_state(&self) -> Result<bool, Error> {
        let resp = self.write(self.cmd.as_slice()).await;
        check_reply(&resp).await?;
        Ok(ascii_to_int(&resp[3..]) == 1)
    }
}

impl SendRecv for DigitalInput {
    fn get_sender(&self) -> &Sender<Message> {
        &self.drive_sender
    }
}
#[derive(Clone, Debug)]
pub struct AnalogInput {
    cmd: [u8; 4],
    drive_sender: Sender<Message>,
}

impl AnalogInput {
    pub fn new(id: u8, drive_sender: Sender<Message>) -> Self {
        let cmd = [STX, b'I', int_to_byte(id), CR];
        Self { cmd, drive_sender }
    }

    pub async fn get_state(&self) -> Result<isize, Error> {
        let res = self.write(self.cmd.as_slice()).await;
        check_reply(&res).await?;
        Ok(ascii_to_int(&res[3..]))
    }
}

impl SendRecv for AnalogInput {
    fn get_sender(&self) -> &Sender<Message> {
        &self.drive_sender
    }
}

#[derive(Clone, Debug)]
pub struct DigitalOutput {
    on_cmd: [u8; 9],
    off_cmd: [u8; 9],
    drive_sender: Sender<Message>,
}

impl DigitalOutput {
    pub fn new(id: u8, drive_sender: Sender<Message>) -> Self {
        let on_cmd = [STX, b'O', int_to_byte(id), b'3', b'2', b'7', b'0', b'0', CR];
        let off_cmd = [STX, b'O', int_to_byte(id), b'0', CR, 0, 0, 0, 0];
        Self {
            on_cmd,
            off_cmd,
            drive_sender,
        }
    }

    fn command_builder(&self, state: bool) -> [u8; 9] {
        if state {
            self.on_cmd
        } else {
            self.off_cmd
        }
    }
    pub async fn set_state(&self, state: bool) -> Result<(), Error> {
        let res = self.write(self.command_builder(state).as_slice()).await;
        check_reply(&res).await?;
        Ok(())
    }
}

impl SendRecv for DigitalOutput {
    fn get_sender(&self) -> &Sender<Message> {
        &self.drive_sender
    }
}

#[derive(Debug, Clone)]
pub enum HBridgeState {
    Pos,
    Neg,
    Off,
}
#[derive(Clone, Debug)]
pub struct HBridge {
    power: i16,
    prefix: [u8; 3],
    drive_sender: Sender<Message>,
}

impl HBridge {
    pub fn new(id: u8, power: i16, drive_sender: Sender<Message>) -> Self {
        let prefix = [STX, b'O', int_to_byte(id)];
        Self {
            power,
            prefix,
            drive_sender,
        }
    }

    fn command_builder(&self, state: HBridgeState) -> Vec<u8> {
        let state = match state {
            HBridgeState::Pos => num_to_bytes(self.power),
            HBridgeState::Neg => num_to_bytes(-self.power),
            HBridgeState::Off => num_to_bytes(0),
        };
        let mut cmd: Vec<u8> = Vec::with_capacity(self.prefix.len() + state.len() + 1);
        cmd.extend_from_slice(self.prefix.as_slice());
        cmd.extend_from_slice(state.as_slice());
        cmd.push(13);
        cmd
    }

    pub async fn set_state(&self, state: HBridgeState) -> Result<(), Error> {
        let resp = self.write(self.command_builder(state).as_slice()).await;
        check_reply(&resp).await?;
        Ok(())
    }
}

impl SendRecv for HBridge {
    fn get_sender(&self) -> &Sender<Message> {
        &self.drive_sender
    }
}
