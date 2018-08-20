use std::sync::mpsc::Sender;
use pub_sub::Subscriber;
use std::rc::Rc;
use pub_sub::PubSubError;

pub struct StdSubscriber<E>{
    sender: Sender<Rc<E>>,
}

impl <E> Subscriber<E> for StdSubscriber<E>{
    fn send(&self, event: Rc<E>) -> Result<bool, PubSubError> {
        match self.sender.send(event) {
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
    use super::*;
    use pub_sub::PubSubChannel;
    use std::sync::mpsc;

    #[derive(Debug, PartialEq)]
    struct TestEvent {}

    #[test]
    pub fn can_subscribe(){
        let (sender, receiver) = mpsc::channel();

        let subscriber = StdSubscriber{
            sender,
        };
        let mut pub_sub = PubSubChannel::new();

        pub_sub.subscribe(subscriber);

        pub_sub.publish(Rc::new(TestEvent{})).unwrap();

        let received_event = receiver.recv().unwrap();
        assert_eq!(TestEvent{}, Rc::try_unwrap(received_event).unwrap());
    }
}