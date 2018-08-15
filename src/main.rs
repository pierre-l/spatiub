extern crate core;

use std::sync::mpsc;
use pub_sub::Event;
use std::sync::mpsc::Sender;
use pub_sub::Subscriber;
use std::rc::Rc;
use pub_sub::PubSubError;
use std::sync::mpsc::SendError;
use pub_sub::PubSubChannel;
use std::error::Error;

mod pub_sub;

fn main() -> Result<(), Box<Error>> {
    let (sender, receiver) = mpsc::channel();

    let subscriber = StdSubscriber{
        id: 0,
        sender,
    };
    let subscriber_id = *subscriber.id();

    let mut pub_sub = PubSubChannel::new();

    pub_sub.subscribe(subscriber);

    pub_sub.publish(Event::Sample)?;

    let received_event = receiver.recv()?;
    assert_eq!(Event::Sample, Rc::try_unwrap(received_event).unwrap());

    pub_sub.unsubscribe(&subscriber_id);

    println!("Hello, world!");
    Ok(())
}

struct StdSubscriber{
    id: u64,
    sender: Sender<Rc<Event>>,
}

impl Subscriber for StdSubscriber{
    fn id(&self) -> &u64 {
        &self.id
    }

    fn send(&self, event: Rc<Event>) -> Result<(), PubSubError> {
        match self.sender.send(event) {
            Ok(()) => {
                Ok(())
            },
            Err(err) => {
                Err(PubSubError::from(err))
            }
        }
    }
}

impl From<SendError<Rc<Event>>> for PubSubError{
    fn from(_: SendError<Rc<Event>>) -> Self {
        PubSubError::ReceiverIsGone
    }
}
