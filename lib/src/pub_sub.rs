use std::rc::Rc;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use core::fmt;
use std::marker::PhantomData;
use uuid::Uuid;

pub struct PubSubChannel<S, E>
    where S: Subscriber<E> {
    subscribers: Vec<S>,
    phantom: PhantomData<E>,
}

impl <S, E> PubSubChannel<S, E> where S: Subscriber<E>{
    pub fn new() -> PubSubChannel<S, E> {
        PubSubChannel{
            subscribers: vec![],
            phantom: PhantomData{},
        }
    }

    pub fn subscribe(&mut self, subscriber: S) {
        self.subscribers.push(subscriber);
    }

    pub fn publish(&mut self, event: Rc<E>) -> Result<(), PubSubError>{
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

pub trait Subscriber<E>: Clone{
    /// Returns Ok(false) or Err to drop the subscription.
    fn send(&self, event: Rc<E>) -> Result<bool, PubSubError>;
    fn entity_id(&self) -> &Uuid;
}

#[derive(Debug)]
pub enum PubSubError{
    ReceiverIsGone,
}

impl Error for PubSubError{}

impl Display for PubSubError{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}