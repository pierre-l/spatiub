use std::collections::HashMap;
use std::rc::Rc;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use core::fmt;

pub struct PubSubChannel<S>
    where S: Subscriber
{
    subscribers: HashMap<u64, S>,
}

impl <S> PubSubChannel<S> where S: Subscriber{
    pub fn new() -> PubSubChannel<S> {
        PubSubChannel{
            subscribers: HashMap::new(),
        }
    }

    pub fn subscribe(&mut self, subscriber: S) {
        let id = *subscriber.id();
        self.subscribers.insert(id, subscriber);
    }

    pub fn unsubscribe(&mut self, subscriber_id: &u64) -> Result<S, PubSubError> {
        if let Some(subscriber) = self.subscribers.remove(subscriber_id) {
            Ok(subscriber)
        } else {
            Err(PubSubError::SubscriptionNotFound)
        }
    }

    pub fn publish(&self, event: Event) -> Result<(), PubSubError>{
        let event = Rc::new(event);
        for subscriber in self.subscribers.values() {
            subscriber.send(event.clone())?;
        }

        Ok(())
    }
}

pub trait Subscriber{
    fn id(&self) -> &u64;
    fn send(&self, event: Rc<Event>) -> Result<(), PubSubError>;
}

#[derive(Debug, PartialEq)]
pub enum Event{
    Sample,
}

#[derive(Debug)]
pub enum PubSubError{
    ReceiverIsGone,
    SubscriptionNotFound,
}

impl Error for PubSubError{}

impl Display for PubSubError{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}