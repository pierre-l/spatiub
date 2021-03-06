use futures::{Future, future, Sink, Stream, stream};
use rand::thread_rng;
use rand::Rng;
use rand::ThreadRng;
use spatiub_demo_core::codec::LengthFieldBasedCodec;
use spatiub::spatial::Entity;
use spatiub::spatial::MapDefinition;
use spatiub::spatial::Point;
use spatiub::spatial::SpatialEvent;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::ops::Add;
use std::time::Duration;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::runtime::current_thread::Runtime;
use tokio::timer::Delay;
use tokio_codec::Decoder;
use uuid::Uuid;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::rc::Rc;
use spatiub_demo_core::message::Message;
use spatiub_demo_core::entity::Timestamp;
use spatiub_demo_core::entity::DemoEntity;
use std::marker::PhantomData;

fn client<C, F>(addr: &SocketAddr, message_consumer: C)
                -> impl Future<Item=(), Error=()>
    where
        C: Fn(Message) -> Option<F>,
        F: Future<Item=Message, Error=()>,
{
    TcpStream::connect(&addr)
        .map_err(|err|{
            panic!("Connection failed. Cause: {}", err)
        })
        .and_then(move |socket| {
            socket.set_nodelay(true).unwrap();
            debug!("Connection established");
            let (output, input) = codec().framed(socket).split();
            let output = output.sink_map_err(|err| error!("An error occurred in the input stream: {}", err));

            input
                .map_err(|err| error!("An error occurred in the input stream: {}", err))
                .map(move |message| {
                    message_consumer(message)
                })
                .filter_map(|future| future)
                .buffered(100000)
                .forward(output)
        })
        .map(|_|{})
}

pub fn run_clients(
    map: &MapDefinition,
    addr: SocketAddr,
    number_of_clients: usize,
    log_file_path: &str,
    msg_per_sec: u64,
) {
    let mut iter = vec![];
    for i in 0..number_of_clients as u64 { iter.push(i) }

    let logger = Rc::new(RefCell::new(ClientEventLogger::new(log_file_path)));
    let mut rng = thread_rng();
    let clients = stream::iter_ok(iter)
        .map(|i| {
            Delay::new(Instant::now().add(Duration::from_millis(i * rng.gen_range(15, 30))))
                .map(|_| {
                    run_client(map.clone(), addr, logger.clone(), msg_per_sec)
                })
                .map_err(|err|{
                    panic!("Timer error: {}", err)
                })
                .flatten()
        })
        .buffered(number_of_clients);

    let mut runtime = Runtime::new().unwrap();
    if let Err(_err) = runtime.block_on(
        clients.for_each(|_| {
            future::ok(())
        })
    ) {
        info!("Client stopped");
    };
}

fn run_client(
    map: MapDefinition,
    addr: SocketAddr,
    logger: Rc<RefCell<ClientEventLogger>>,
    msg_per_sec: u64,
) -> impl Future<Item=(), Error=()> {
    let ref addr = addr;
    let client_entity_id = RefCell::new(None);

    client(
        &addr,
        move |message| {
            if let Message::ConnectionAck(entity) = &message {
                client_entity_id.replace(Some(entity.id().clone()));
            } else if let Message::Event(event) = &message {
                let latency = event.acting_entity.last_state_update.elapsed();

                logger.borrow_mut().log(event.clone(), latency);
            };

            if let Some(ref entity_id) = &*client_entity_id.borrow() {
                trigger_new_move_if_client_entity_involved(
                    message,
                    &map,
                    entity_id,
                    msg_per_sec
                )
            } else {
                panic!("Expected the entity id to be set");
            }
        })
}

fn trigger_new_move_if_client_entity_involved(
    message: Message,
    map: &MapDefinition,
    client_entity_id: &Uuid,
    msg_per_sec: u64,
)
    -> Option<impl Future<Item=Message, Error=()>>
{
    // PERFORMANCE Suboptimal. Is there a way to avoid calling thread_rng everytime?
    let mut rng = thread_rng();

    if let Message::Event(
        SpatialEvent{
            from: _,
            to: Some(to),
            acting_entity,
            is_a_move: true,
        }
    ) = message{
        if acting_entity.id() == client_entity_id {
            let delayed_move = trigger_new_move(&mut rng, &map, acting_entity, to, msg_per_sec);

            Some(delayed_move)
        } else {
            None
        }
    } else {
        None
    }
}

fn trigger_new_move(
    rng: &mut ThreadRng,
    map: &MapDefinition,
    mut entity: DemoEntity,
    from: Point,
    msg_per_sec: u64,
) -> impl Future<Item=Message, Error=()> {
    let next_destination = map.random_point_next_to(&from, rng);
    Delay::new(Instant::now().add(Duration::from_nanos(rng.gen_range(500_000_000/msg_per_sec, 1_500_000_000/msg_per_sec))))
        .map(move |()| {
            entity.last_state_update = Timestamp::new();

            Message::Event(SpatialEvent {
                from,
                to: Some(next_destination),
                acting_entity: entity,
                is_a_move: true,
            })
        })
        .map_err(|err|{
            panic!("Timer error: {}", err)
        })
}

struct ClientEventLogger {
    writer: BufWriter<File>,
}

impl ClientEventLogger{
    pub fn new(filepath: &str) -> ClientEventLogger {
        let file = File::create(filepath).expect("Could not open the file.");
        let writer = BufWriter::new(file);

        ClientEventLogger{
            writer,
        }
    }

    pub fn log(&mut self, event: SpatialEvent<DemoEntity>, latency: Duration){
        let entry = format!("{},{}\n", latency.subsec_nanos(), event.acting_entity.last_state_update);
        self.writer.write(entry.as_bytes()).expect("Could not write to the file.");
    }
}

pub fn codec() -> LengthFieldBasedCodec<Message> {
    LengthFieldBasedCodec{
        phantom: PhantomData,
    }
}