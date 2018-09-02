use codec::LengthFieldBasedCodec;
use entity::DemoEntity;
use futures::{Future, future, Stream, stream, Sink};
use spatiub::futures_sub;
use spatiub::spatial::Entity;
use spatiub::spatial::MapDefinition;
use spatiub::spatial::Point;
use spatiub::spatial::SpatialChannel;
use tokio_codec::Decoder;
use tokio::net::TcpListener;
use tokio::runtime::current_thread::Runtime;
use uuid::Uuid;
use spatiub::spatial::SpatialEvent;
use std::io::Error;
use std::marker::PhantomData;
use spatiub::futures_sub::FutureSubscriber;
use message::Message;
use futures::unsync::mpsc::UnboundedReceiver;
use std::rc::Rc;
use std::cell::RefCell;
use std::net::SocketAddr;
use entity::Timestamp;

type Event = SpatialEvent<DemoEntity>;
type SpatialChannelCell = RefCell<SpatialChannel<FutureSubscriber<Event>, DemoEntity>>;

pub fn server(addr: &SocketAddr) {
    let channel = RefCell::new(channel());

    let mut runtime = Runtime::new().unwrap();

    let listener = TcpListener::bind(&addr).unwrap();

    let server = listener.incoming().map(|socket| {
        let (output, input) = codec().framed(socket).split();

        let entity = DemoEntity{
            id: Uuid::new_v4(),
            last_state_update: Timestamp::new(),
        };

        let (subscriber, subscription) = futures_sub::new_subscriber(entity.id().clone());

        let position = Point(0, 0);
        subscribe(&channel, subscriber, &position);

        publish(&channel, Event{
            to: Some(position.clone()),
            from: position,
            acting_entity: entity.clone(),
            is_a_move: true,
        });

        outgoing_events(subscription, entity, output)
            .join(
                input
                    .map_err(|err|{
                        error!("IO error in the input stream: {}", err)
                    })
                    .for_each(|message|{
                        match message {
                            Message::Event(event) => {
                                publish(&channel, event);

                                future::ok(())
                            },
                            Message::ConnectionAck(_) => {
                                // Forbidden for clients
                                future::err(())
                            }
                        }
                    }))
    })
        .map_err(|err| {
            error!("An unexpected error occurred: {}", err);
        })
        .buffered(100000)
        .for_each(|_|{
            Ok(())
        })
    ;

    runtime.block_on(server).unwrap();

    info!("Server stopped");
}

fn channel() -> SpatialChannel<FutureSubscriber<SpatialEvent<DemoEntity>>, DemoEntity> {
    SpatialChannel::new(MapDefinition {
        zone_width: 16,
        map_width_in_zones: 16 * 16,
    })
}

fn publish(
    channel: &SpatialChannelCell,
    event: Event,
) {
    match channel.try_borrow_mut(){
        Ok(mut channel_ref) => {
            channel_ref.publish(event);
        },
        Err(err) => {
            panic!("Could not publish {:?}. Cause: {}", event, err)
        }
    }
}

fn subscribe(
    channel: &SpatialChannelCell,
    subscriber: FutureSubscriber<Event>,
    position: &Point,
) {
    match channel.try_borrow_mut(){
        Ok(mut channel_ref) => {
            channel_ref.subscribe(subscriber, position);
        },
        Err(err) => {
            panic!("Could not subscribe {:?} at {:?}. Cause: {}", subscriber, position, err)
        }
    }
}

pub fn codec() -> LengthFieldBasedCodec<Message> {
    LengthFieldBasedCodec{
        phantom: PhantomData,
    }
}

fn outgoing_events<S>(
    subscription_stream: UnboundedReceiver<Rc<Event>>,
    entity: DemoEntity,
    sender: S,
) -> impl Future<Item=(), Error=()>
    where S: Sink<SinkItem=Message, SinkError=Error>,
{
    let connection_ack = stream::once(Ok(Message::ConnectionAck(entity)));
    let outgoing_events =
        connection_ack.chain(subscription_stream
            .map(|event|{
                Message::Event(event.as_ref().clone())
            })
        )
            .forward(sender
                .sink_map_err(|err|{
                    error!("IO error in the output stream: {}", err)
                }));

    outgoing_events
        .map(|_|{})
        .map_err(|_|{})
}