use messr::Router;
use tokio::sync::{broadcast::Receiver, mpsc::Sender};

pub use crate::{event::*, event_data::*};

mod event;
mod event_data;

pub const DEFAULT_BUFFER: usize = 1000;

pub type EventRouter = Router<Event>;
pub type EventMessage = messr::Message<Event>;
pub type EventPublisher = Sender<EventMessage>;
pub type EventSubscriber = Receiver<EventMessage>;
pub type Topic = messr::Topic;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn _event_can_turn_into_router_message() {
        let event = Event::NoOp;
        let message: messr::Message<Event> = event.into();

        assert_eq!(
            message,
            messr::Message::new_with_id(message.id, Event::NoOp, None)
        );
    }
}
