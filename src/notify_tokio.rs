use notify::{Event, EventHandler};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct TokioEventHandler(UnboundedSender<notify::Result<Event>>);

impl TokioEventHandler {
    pub fn unbounded() -> (Self, UnboundedReceiver<notify::Result<Event>>) {
        let (sender, receiver): (
            UnboundedSender<notify::Result<Event>>,
            UnboundedReceiver<notify::Result<Event>>,
        ) = tokio::sync::mpsc::unbounded_channel();
        let handler = TokioEventHandler(sender);
        (handler, receiver)
    }
}

impl EventHandler for TokioEventHandler {
    fn handle_event(&mut self, event: notify::Result<Event>) {
        match self.0.send(event) {
            Ok(_) => {}
            Err(e) => println!("Error sending event: {:?}", e),
        }
    }
}
