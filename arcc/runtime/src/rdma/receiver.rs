extern crate infinity;

use kompact::*;
use std::sync::Arc;
use crate::rdma::RdmaContext;

#[derive(ComponentDefinition)]
pub struct Task {
    ctx: ComponentContext<Task>,
    rdma_ctx: Box<RdmaContext>,
}

impl Task {
    pub fn new() -> Task {
        Task {
            ctx: ComponentContext::new(),
            rdma_ctx: Box::new(RdmaContext::new()),
        }
    }
}

impl Provide<ControlPort> for Task {
    fn handle(&mut self, event: ControlEvent) -> () {
        if let ControlEvent::Start = event {}
    }
}

impl Actor for Task {
    fn receive_local(&mut self, _sender: ActorRef, msg: &Any) {}
    fn receive_message(&mut self, _sender: ActorPath, ser_id: u64, buf: &mut Buf) {}
}
