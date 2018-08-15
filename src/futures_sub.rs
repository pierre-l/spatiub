use futures::{Stream, Future};
use futures::unsync::mpsc::{self, UnboundedReceiver};
use futures::unsync::mpsc::UnboundedSender;
use pub_sub::Event;
use pub_sub::PubSubError;
use pub_sub::Subscriber;
use std::rc::Rc;

pub struct FutureSubscriber {
    id: u64,
    sender: UnboundedSender<Rc<Event>>,
}

pub fn new_channel(id: u64) -> (FutureSubscriber, UnboundedReceiver<Rc<Event>>) {
    let (sender, receiver) = mpsc::unbounded();

    let subscriber = FutureSubscriber {
        id,
        sender,
    };

    (subscriber, receiver)
}

impl Subscriber for FutureSubscriber {
    fn id(&self) -> &u64 {
        &self.id
    }

    fn send(&self, event: Rc<Event>) -> Result<(), PubSubError> {
        match &self.sender.unbounded_send(event) {
            Ok(()) => {
                Ok(())
            },
            Err(err) => {
                Err(PubSubError::ReceiverIsGone)
            }
        }
    }
}

#[cfg(test)]
mod tests{
    use pub_sub::PubSubChannel;
    use std::sync::mpsc;
    use super::*;

    #[test]
    pub fn can_subscribe(){
        let (subscriber, receiver) = super::new_channel(0);
        let subscriber_id = *subscriber.id();

        let mut pub_sub = PubSubChannel::new();

        pub_sub.subscribe(subscriber);

        pub_sub.publish(Event::Sample).unwrap();

        let (received_event_option, _receiver) = receiver.into_future().wait().unwrap();
        assert_eq!(Event::Sample, Rc::try_unwrap(received_event_option.unwrap()).unwrap());

        pub_sub.unsubscribe(&subscriber_id).unwrap();
    }
}