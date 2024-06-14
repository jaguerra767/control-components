use std::error::Error;
use crate::util::utils::{ascii_to_int, int_to_byte, int_to_bytes};
use crate::controllers::clear_core::{STX, CR, Controller};

#[allow(unused)]
const CLEAR_CORE_H_BRIDGE_MAX: i16 = 32760;

pub struct DigitalInput<'a>{
    cmd: [u8;4],
    drive: &'a Controller
}

impl <'a> DigitalInput<'a> {
    pub fn new(id: u8, drive: &'a Controller) -> Self {
        let cmd = [STX, b'I', int_to_byte(id), CR];
        Self{ cmd, drive }
    }

    pub async fn get_state(&self) -> Result<bool, Box<dyn Error>> {
         let res = self.drive.write(self.cmd.as_slice()).await?;
        Ok(ascii_to_int(&res[3..]) == 1)
    }
}

pub struct AnalogInput <'a>{
    cmd: [u8;4],
    drive: &'a Controller
}

impl <'a> AnalogInput <'a> {
    pub fn new(id: u8, drive: &'a Controller) -> Self {
        let cmd = [STX, b'I', int_to_byte(id), CR];
        Self{ cmd, drive }
    }

    pub async fn get_state(&self) -> Result<isize, Box<dyn Error>> {
        let res = self.drive.write(self.cmd.as_slice()).await?;
        Ok(ascii_to_int(&res[3..]))
    }
}


pub enum OutputState {
    Off,
    On
}

pub struct Output<'a>{
    on_cmd: [u8; 9],
    off_cmd: [u8; 9],
    drive: &'a Controller
}

impl <'a> Output <'a> {
    pub fn new(id: u8, drive: &'a Controller) -> Self {
        let on_cmd = [STX, b'O', int_to_byte(id), b'3' , b'2', b'7', b'0', b'0', CR];
        let off_cmd = [STX, b'O', int_to_byte(id), b'0', CR, 0, 0, 0, 0];
        Self{on_cmd, off_cmd, drive}
    }

    fn command_builder(&self, state: OutputState) -> [u8;9] {
        match state {
            OutputState::Off => {self.off_cmd}
            OutputState::On => {self.on_cmd}
        }
    }

    pub async fn set_state(&self, state: OutputState) -> Result<isize, Box<dyn Error>> {
        let res = self.drive.write(self.command_builder(state).as_slice()).await?;
        Ok(ascii_to_int(&res[3..]))
    }
}

#[derive(Debug)]
pub enum HBridgeState {
    Pos,
    Neg,
    Off
}

pub struct HBridge <'a> {
    power: i16,
    prefix: [u8;3],
    drive: &'a Controller
}

impl <'a> HBridge <'a> {
    pub fn new(id: u8, power: i16, drive: &'a Controller) -> Self {
        let prefix = [STX, b'O', int_to_byte(id)];
        Self{power, prefix, drive}
    }

    fn command_builder(&self, state: HBridgeState) -> Vec<u8> {
        let state = match state {
            HBridgeState::Pos => {int_to_bytes(self.power)}
            HBridgeState::Neg => {int_to_bytes(-self.power)}
            HBridgeState::Off => {int_to_bytes(0)}
        };
        let mut cmd: Vec<u8> = Vec::with_capacity(self.prefix.len() + state.len() + 1);
        cmd.extend_from_slice(self.prefix.as_slice());
        cmd.extend_from_slice(state.as_slice());
        cmd.push(13);
        cmd
    }

    pub async fn set_state(&self, state: HBridgeState) -> Result<(), Box<dyn Error>> {
        self.drive.write(self.command_builder(state).as_slice()).await?;
        Ok(())
    }
}
