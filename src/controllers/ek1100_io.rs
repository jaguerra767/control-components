use std::error::Error;
use ethercrab::std::{ethercat_now, tx_rx_task};
use ethercrab::{Client, ClientConfig, PduStorage, Timeouts};
use log::info;
use std::future::Future;
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
    pub cmd: Command
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
        let msg = EthCmd {card_id, idx, cmd: Command::SetState(state)};
        self.tx.send(msg).await.unwrap();
    }
}
#[derive(Clone)]
pub struct Controller {
    io: Vec<IOCard>,
}

impl Controller {
    pub fn new(tx: Sender<EthCmd>, io_qty: u8) -> Self {
        let io: Vec<IOCard> = (0..io_qty).map(|_| IOCard::new(tx.clone())).collect();
        Self { io }
    }

    pub fn with_client(
        interface: &'static str, io_qty: u8
    ) -> (Self, impl Future<Output = Result<(), Box<dyn Error + Send + Sync>>>) {
        let (tx, rx) = channel(100);
        (Self::new(tx, io_qty), client(interface, rx))
    }

    pub fn get_io(&mut self, card_id: usize) -> IOCard {
        self.io[card_id].clone()
    }
}

pub async fn client(interface: &str, mut rx: Receiver<EthCmd>) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Hello from client");
    let (pdu_tx, pdu_rx, pdu_loop) = PDU_STORAGE
        .try_split()
        .expect("This method can only be called once"); //.expect("Can only split once");

    let client = Arc::new(Client::new(
        pdu_loop,
        Timeouts::default(),
        ClientConfig::default(),
    ));

    info!("Starting EtherCAT master");
    tokio::spawn(tx_rx_task(interface, pdu_tx, pdu_rx).expect("spawn TX/RX task"));

    let group = client
        .init_single_group::<MAX_SLAVES, PDI_LEN>(ethercat_now)
        .await
        .expect("Init");

    info!("Discovered {} slaves", group.len());
    info!("Initialized EtherCAT Group");

    let mut group = group.into_op(&client).await.expect("PRE-OP -> OP");
    info!("PRE-OP -> OP");

    for slave in group.iter(&client) {
        let (i, o) = slave.io_raw();

        info!(
            "-> Slave {:#06x} {} inputs: {} bytes, outputs: {} bytes",
            slave.configured_address(),
            slave.name(),
            i.len(),
            o.len()
        );
    }

    let mut tick_interval = tokio::time::interval(Duration::from_millis(1));
    tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let shutdown = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown))
        .expect("Register hook");

    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        group.tx_rx(&client).await.expect("Tx/Rx");

        match rx.try_recv() {
            Ok(msg) => {
                let card_id = msg.card_id;
                let mut slave = group.slave(&client, card_id).expect("Unable to get slave");
                let (i, o) = slave.io_raw_mut();
                match msg.cmd {
                    Command::SetState(state) => {
                        info!("SetState with new state: {state} called on EK1100 card: {card_id}");
                        let old_state = o[0];
                        let shift = msg.idx;
                        o[0] = old_state & !(1 << shift) | (u8::from(state) << shift);
                    }
                    Command::GetState(tx) => {
                        let state = i[0];
                        info!("GetState with response: {state} called on EK1100 card: {card_id}");
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

    let group = group.into_safe_op(&client).await.expect("OP -> SAFE-OP");

    info!("OP -> SAFE-OP");

    let group = group.into_pre_op(&client).await.expect("SAFE-OP -> PRE-OP");

    info!("SAFE-OP -> PRE-OP");

    let _group = group.into_init(&client).await.expect("PRE-OP -> INIT");

    info!("PRE-OP -> INIT, shutdown complete");
    Ok(())
}

#[tokio::test]
async fn test_ek1100() {
    use env_logger::Env;
    use tokio::join;

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let interface = "enp1s0f0";
    let (tx, rx) = channel(10);

    let client_handler = tokio::spawn(client(interface, rx));

    let mut controller = Controller::new(tx, 1);
    tokio::time::sleep(Duration::from_secs(1)).await;
    let task = tokio::spawn(async move {
        info!("Hello from tasky task");
        let mut io = controller.get_io(0);
        for i in 0..8 {
            io.set_state(1, i, true).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
            io.set_state(1, i, false).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    let _ = join!(client_handler, task);
}
