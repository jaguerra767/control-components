use ethercrab::std::{ethercat_now, tx_rx_task};
use ethercrab::{MainDevice, MainDeviceConfig, PduStorage, Timeouts};
use log::info;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::time::{Duration, MissedTickBehavior};

const MAX_SLAVES: usize = 16;
const MAX_PDU_DATA: usize = PduStorage::element_size(1100);
const MAX_FRAMES: usize = 16;
const PDI_LEN: usize = 64;

static PDU_STORAGE: PduStorage<MAX_FRAMES, MAX_PDU_DATA> = PduStorage::new();

pub enum Command {
    SetState(bool),
    GetState(oneshot::Sender<u8>),
}

pub struct EthCmd {
    pub card_id: usize,
    pub idx: u8,
    pub cmd: Command,
}

#[derive(Clone, Debug)]
pub struct IOCard {
    tx: Sender<EthCmd>,
}

impl IOCard {
    pub fn new(tx: Sender<EthCmd>) -> Self {
        Self { tx }
    }
    pub async fn set_state(&mut self, card_id: usize, idx: u8, state: bool) {
        let msg = EthCmd {
            card_id,
            idx,
            cmd: Command::SetState(state),
        };
        self.tx.send(msg).await.unwrap();
    }
}
#[derive(Clone)]
pub struct ControllerHandle {
    io: Vec<IOCard>,
}

impl ControllerHandle {
    pub fn new(interface: &'static str, io_qty: u8) -> Self {
        let (tx, rx) = channel(100);
        let io: Vec<IOCard> = (0..io_qty).map(|_| IOCard::new(tx.clone())).collect();
        tokio::spawn(async move {
            client(interface, rx).await.unwrap();
        });
        Self { io }
    }

    pub fn get_io(&mut self, card_id: usize) -> IOCard {
        self.io[card_id].clone()
    }
}

async fn client(
    interface: &str,
    mut rx: Receiver<EthCmd>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Hello from client");
    let (pdu_tx, pdu_rx, pdu_loop) = PDU_STORAGE
        .try_split()
        .expect("This method can only be called once"); //.expect("Can only split once");

    let main_device = Arc::new(MainDevice::new(
        pdu_loop,
        Timeouts {
            wait_loop_delay: Duration::from_millis(2),
            mailbox_response: Duration::from_millis(1000),
            ..Default::default()
        },
        MainDeviceConfig::default(),
    ));

    info!("Starting EtherCAT master");
    tokio::spawn(tx_rx_task(interface, pdu_tx, pdu_rx).expect("spawn TX/RX task"));

    let group = main_device
        .init_single_group::<MAX_SLAVES, PDI_LEN>(ethercat_now)
        .await
        .expect("Init");

    info!("Discovered {} slaves", group.len());
    info!("Initialized EtherCAT Group");

    let mut group = group.into_op(&main_device).await.expect("PRE-OP -> OP");
    info!("PRE-OP -> OP");

    for slave in group.iter(&main_device) {
        let (i, o) = slave.io_raw();

        info!(
            "-> Slave {:#06x} {} inputs: {} bytes, outputs: {} bytes",
            slave.configured_address(),
            slave.name(),
            i.len(),
            o.len()
        );
    }

    let mut tick_interval = tokio::time::interval(Duration::from_millis(5));
    tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let shutdown = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown))
        .expect("Register hook");

    loop {
        if shutdown.load(Ordering::Relaxed) {
            info!("Shutting down EK1100 client");
            break;
        }
        group.tx_rx(&main_device).await.expect("Tx/Rx");

        match rx.try_recv() {
            Ok(msg) => {
                let card_id = msg.card_id;
                let mut sub_device = group
                    .subdevice(&main_device, card_id)
                    .expect("Unable to get sub-device");
                let (i, o) = sub_device.io_raw_mut();
                match msg.cmd {
                    Command::SetState(state) => {
                        let old_state = o[0];
                        let shift = msg.idx;
                        o[0] = old_state & !(1 << shift) | (u8::from(state) << shift);
                    }
                    Command::GetState(tx) => {
                        let state = i[0];
                        tx.send(state).unwrap();
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
                info!("All senders dropped, Disconnecting");
                break;
            }
            Err(_) => {}
        }
        tick_interval.tick().await;
    }

    let group = group
        .into_safe_op(&main_device)
        .await
        .expect("OP -> SAFE-OP");

    info!("OP -> SAFE-OP");

    let group = group
        .into_pre_op(&main_device)
        .await
        .expect("SAFE-OP -> PRE-OP");

    info!("SAFE-OP -> PRE-OP");

    let _group = group.into_init(&main_device).await.expect("PRE-OP -> INIT");

    info!("PRE-OP -> INIT, shutdown complete");
    Ok(())
}

// #[tokio::test]
// async fn test_ek1100() {
//     use env_logger::Env;
//     use tokio::join;
//
//     env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
//     let interface = "enp1s0f0";
//     let (tx, rx) = channel(10);
//
//     let client_handler = tokio::spawn(client(interface, rx));
//
//     let mut controller = Controller::new(tx, 1);
//     tokio::time::sleep(Duration::from_secs(1)).await;
//     let task = tokio::spawn(async move {
//         info!("Hello from tasky task");
//         let mut io = controller.get_io(0);
//         for i in 0..8 {
//             io.set_state(1, i, true).await;
//             tokio::time::sleep(Duration::from_secs(1)).await;
//             io.set_state(1, i, false).await;
//             tokio::time::sleep(Duration::from_secs(1)).await;
//         }
//     });
//
//     let _ = join!(client_handler, task);
// }
