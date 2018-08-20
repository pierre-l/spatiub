use futures::unsync::mpsc::{self, UnboundedReceiver};
use futures::unsync::mpsc::UnboundedSender;
use pub_sub::PubSubError;
use pub_sub::Subscriber;
use std::rc::Rc;

pub struct FutureSubscriber<E> {
    sender: UnboundedSender<Rc<E>>,
}

pub fn new_subscriber<E>() -> (FutureSubscriber<E>, UnboundedReceiver<Rc<E>>) {
    let (sender, receiver) = mpsc::unbounded();

    let subscriber = FutureSubscriber {
        sender,
    };

    (subscriber, receiver)
}

impl <E> Subscriber<E> for FutureSubscriber<E> {
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
}

#[cfg(test)]
mod tests{
    use pub_sub::PubSubChannel;
    use futures::{Stream, Future};
    use super::*;

    #[derive(Debug, PartialEq)]
    struct TestEvent {}

    #[test]
    pub fn can_subscribe(){
        let (subscriber, receiver) = super::new_subscriber();
        let mut pub_sub = PubSubChannel::new();

        pub_sub.subscribe(subscriber);

        pub_sub.publish(Rc::new(TestEvent {})).unwrap();

        let (received_event_option, _receiver) = receiver.into_future().wait().unwrap();
        assert_eq!(TestEvent {}, Rc::try_unwrap(received_event_option.unwrap()).unwrap());
    }
}