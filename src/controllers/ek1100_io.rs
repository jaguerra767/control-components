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
    SetState(u8),
    GetState(oneshot::Sender<u8>),
}
pub struct Message {
    pub card_id: usize,
    pub card_state: u8,
    pub command: Command,
}

pub struct IOCard {
    state: u8,
    tx: Sender<Message>,
}

impl IOCard {
    pub fn new(tx: Sender<Message>) -> Self {
        Self { state: 0, tx }
    }

    pub async fn set_state(&mut self, card_id: usize, idx: u8, state: bool) {
        let shift = idx;
        self.state = self.state & !(1 << shift) | (u8::from(state) << shift);
        let msg = Message {
            card_id,
            card_state: self.state,
            command: Command::SetState(self.state),
        };
        self.tx.send(msg).await.unwrap();
    }
}

pub struct Controller {
    io: Vec<IOCard>,
}

impl Controller {
    pub fn new(tx: Sender<Message>, io_qty: u8) -> Self {
        let io: Vec<IOCard> = (0..io_qty).map(|_| IOCard::new(tx.clone())).collect();
        Self { io }
    }

    pub fn with_client(interface: &str, io_qty: u8) -> (Self, impl Future<Output = ()> + '_) {
        let (tx, rx) = channel(100);
        (Self::new(tx, io_qty), client(interface, rx))
    }

    pub fn get_io(&mut self, card_id: usize) -> Option<&mut IOCard> {
        self.io.get_mut(card_id)
    }
}

pub async fn client(interface: &str, mut rx: Receiver<Message>) {
    println!("Hello from client");
    let (pdu_tx, pdu_rx, pdu_loop) = PDU_STORAGE
        .try_split()
        .expect("This method can only be called once"); //.expect("Can only split once");

    let client = Arc::new(Client::new(
        pdu_loop,
        Timeouts {
            wait_loop_delay: Duration::from_millis(2),
            mailbox_response: Duration::from_millis(1000),
            ..Default::default()
        },
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

    let mut tick_interval = tokio::time::interval(Duration::from_millis(5));
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
                let mut slave = group.slave(&client, msg.card_id).unwrap();
                let (i, o) = slave.io_raw_mut();
                match msg.command {
                    Command::SetState(state) => {
                        o[0] = state;
                    }
                    Command::GetState(tx) => {
                        tx.send(i[0]).unwrap();
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
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
}

#[tokio::test]
async fn test_ek1100() {
    use env_logger::Env;
    use log::error;
    use tokio::join;

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let interface = "enp1s0f0";
    let (tx, rx) = channel(10);

    let client_handler = tokio::spawn(client(interface, rx));

    let mut controller = Controller::new(tx, 1);
    tokio::time::sleep(Duration::from_secs(1)).await;
    let task = tokio::spawn(async move {
        info!("Hello from tasky task");
        if let Some(io) = controller.get_io(0) {
            for i in 0..8 {
                io.set_state(1, i, true).await;
                tokio::time::sleep(Duration::from_secs(1)).await;
                io.set_state(1, i, false).await;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        } else {
            error!("Failed to get IO");
        }
    });

    let _ = join!(client_handler, task);
}
