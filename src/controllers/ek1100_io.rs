use ethercrab::std::{ethercat_now, tx_rx_task};
use ethercrab::{MainDevice, MainDeviceConfig, PduStorage, SubDevicePdi, SubDeviceRef, Timeouts};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, MissedTickBehavior};

const MAX_SLAVES: usize = 16;
const MAX_PDU_DATA: usize = PduStorage::element_size(1100);
const MAX_FRAMES: usize = 16;
const PDI_LEN: usize = 64;

static PDU_STORAGE: PduStorage<MAX_FRAMES, MAX_PDU_DATA> = PduStorage::new();

struct Ek1100Client {
    receiver: Receiver<IoMsg>,
    states: Vec<u8>,
}

enum Command {
    SetState(bool),
    GetState(oneshot::Sender<u8>),
}

struct IoMsg {
    pub slot: usize,
    pub idx: u8,
    pub cmd: Command,
}

impl Ek1100Client {
    fn new(receiver: Receiver<IoMsg>) -> Self {
        let states = Vec::with_capacity(MAX_SLAVES);
        Self { receiver, states }
    }

    fn handle_message(&mut self, msg: IoMsg, sub_device: &mut SubDeviceRef<SubDevicePdi>) {
        match msg.cmd {
            Command::SetState(io) => { 
                let (_, o) = sub_device.io_raw_mut();
                let shift = msg.idx;
                self.states[msg.slot] = o[0];
                o[0] = self.states[msg.slot] & !(1 << shift) | (u8::from(io) << shift);
            }
            Command::GetState(rep) => {
                let (i, _) = sub_device.io_raw();
                rep.send(i[0]).unwrap();
            }
        }
    }
}

async fn run_client(interface: &str, mut client: Ek1100Client) {
    let (pdu_tx, pdu_rx, pdu_loop) = PDU_STORAGE.try_split().unwrap();
    let main_device = Arc::new(MainDevice::new(
        pdu_loop,
        Timeouts::default(),
        MainDeviceConfig::default(),
    ));
    tokio::spawn(tx_rx_task(interface, pdu_tx, pdu_rx).expect("spawn TX/RX task"));
    let group = main_device
        .init_single_group::<MAX_SLAVES, PDI_LEN>(ethercat_now)
        .await
        .expect("Init");
    let group = group.into_op(&main_device).await.expect("PRE-OP -> OP");
    let mut tick_interval = tokio::time::interval(Duration::from_secs(1));
    tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let shutdown = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown))
        .expect("Register hook");

    while !shutdown.load(Ordering::Relaxed) {
        group.tx_rx(&main_device).await.expect("Tx/Rx");

        match client.receiver.try_recv() {
            Ok(msg) => {
                let mut sub_device = group.subdevice(&main_device, msg.slot)
                    .expect("Unable to get sub device");
                client.handle_message(msg, &mut sub_device)
            },
            Err(e) => {
                if e == TryRecvError::Disconnected {
                    break;
                }
            }
        }
        tick_interval.tick().await;
    }
}

#[derive(Clone)]
pub struct Ek1100Handler{
    sender: Sender<IoMsg>
}

impl Ek1100Handler {
    pub fn new(interface: &'static str) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        let client = Ek1100Client::new(receiver);
        tokio::spawn(async move {
            run_client(interface, client).await;
        });
        Self{sender}
    }
    
    pub async fn set_state(&self, slot: usize, idx: u8,  state: bool) {
        let msg = IoMsg { slot, idx,  cmd: Command::SetState(state)};
        self.sender.send(msg).await.unwrap();
    }
    
    pub async fn get_state(&self, slot: usize, idx: u8) -> bool {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = IoMsg {slot, idx, cmd: Command::GetState(resp_tx)};
        self.sender.send(msg).await.unwrap();
        resp_rx.await.unwrap() == 1
    }
}

//     for slave in group.iter(&main_device) {
//         let (i, o) = slave.io_raw();
//
//         info!(
//             "-> Slave {:#06x} {} inputs: {} bytes, outputs: {} bytes",
//             slave.configured_address(),
//             slave.name(),
//             i.len(),
//             o.len()
//         );
//     }
//


