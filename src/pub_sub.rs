use std::collections::HashMap;
use std::rc::Rc;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use core::fmt;

pub struct PubSubChannel<S>
    where S: Subscriber {
    subscribers: Vec<S>,
}

impl <S> PubSubChannel<S> where S: Subscriber{
    pub fn new() -> PubSubChannel<S> {
        PubSubChannel{
            subscribers: vec![],
        }
    }

    pub fn subscribe(&mut self, subscriber: S) {
        self.subscribers.push(subscriber);
    }

    pub fn publish(&mut self, event: Event) -> Result<(), PubSubError>{
        let event = Rc::new(event);

        self.subscribers.retain(|subscriber|{
            match subscriber.send(event.clone()) {
                Ok(retain) => retain,
                Err(err) => {
                    warn!("Subscriber dropped. Cause: {}", err);
                    false
                }
            }
        });

        Ok(())
    }
}

pub trait Subscriber{
    /// Returns Ok(false) to
    fn send(&self, event: Rc<Event>) -> Result<bool, PubSubError>;
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