use futures::unsync::mpsc::{self, UnboundedReceiver};
use futures::unsync::mpsc::UnboundedSender;
use pub_sub::PubSubError;
use pub_sub::Subscriber;
use std::rc::Rc;
use uuid::Uuid;

#[derive(Clone)]
pub struct FutureSubscriber<E: Clone> {
    sender: UnboundedSender<Rc<E>>,
    entity_id: Uuid,
}

pub fn new_subscriber<E: Clone>(entity_id: Uuid) -> (FutureSubscriber<E>, UnboundedReceiver<Rc<E>>) {
    let (sender, receiver) = mpsc::unbounded();

    let subscriber = FutureSubscriber {
        sender,
        entity_id,
    };

    (subscriber, receiver)
}

impl <E: Clone> Subscriber<E> for FutureSubscriber<E> {
    fn send(&self, event: Rc<E>) -> Result<bool, PubSubError> {
        match &self.sender.unbounded_send(event) {
            Ok(()) => {
                Ok(true)
            },
            Err(_err) => {
                Err(PubSubError::ReceiverIsGone)
            }
        }
    }

    fn entity_id(&self) -> &Uuid {
        &self.entity_id
    }
}

#[cfg(test)]
mod tests{
    use pub_sub::PubSubChannel;
    use futures::{Stream, Future};
    use super::*;

    #[derive(Debug, PartialEq, Clone)]
    struct TestEvent {}

    #[test]
    pub fn can_subscribe(){
        let (subscriber, receiver) = super::new_subscriber(Uuid::new_v4());
        let mut pub_sub = PubSubChannel::new();

        pub_sub.subscribe(subscriber);

        pub_sub.publish(Rc::new(TestEvent {})).unwrap();

        let (received_event_option, _receiver) = receiver.into_future().wait().unwrap();
        assert_eq!(TestEvent {}, Rc::try_unwrap(received_event_option.unwrap()).unwrap());
    }
}