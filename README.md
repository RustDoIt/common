# Common
This library contains all the code that is common between each component (clients and servers).

## How to Instantiate a generic Node (client or server)
It follows an example implementation of a generic Node (in this case a Server):

```rust
pub struct Server {
    routing_handler: RoutingHandler,
    received_messages: Vec<(String, String)>,
    communication_server: Vec<NodeId>,
    controller_recv: Receiver<Box<dyn Any>>,
    packet_recv: Receiver<Packet>,
    assembler: FragmentAssembler,
    ...
}

impl Server {
    #[must_use]
    pub fn new(
        id: NodeId,
        neighbors: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_recv: Receiver<Box<dyn Any>>,
        controller_send: Sender<Box<dyn Any>>,
        ...
    ) -> Self {
        let routing_handler = RoutingHandler::new(id, NodeType::Client, neighbors, controller_send);

        Self {
            routing_handler,
            received_messages: vec![],
            communication_server: vec![],
            controller_recv,
            packet_recv,
            assembler: FragmentAssembler::default()
            ...
        }
    }
}


impl Processor for Server {
    fn controller_recv(&self) -> &Receiver<Box<dyn Any>> {
        &self.controller_recv
    }

    fn packet_recv(&self) -> &Receiver<Packet> {
        &self.packet_recv
    }

    fn handle_command(&mut self, cmd: Box<dyn Any>) {
        if let Ok(cmd) = cmd.downcast::<ServerCommand>() {
            match *cmd {
                // match on server command
            }
        }

    }

    fn assembler(&mut self) -> &mut FragmentAssembler {
        &mut self.assembler
    }

    fn routing_header(&mut self) -> &mut RoutingHandler {
        &mut self.routing_handler
    }

    fn handle_msg(&mut self, msg: Vec<u8>) {

        if let Ok(msg) = serde_json::from_slice::<ClientRequest>(&msg) {
            match msg {
                // match on ClientRequest
            }
        }
    }
}
```
