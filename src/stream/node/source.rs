// Copyright (c) 2020, KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    data::{ArconEvent, Epoch, Watermark},
    manager::source::{SourceManagerEvent, SourceManagerPort},
    stream::{
        channel::strategy::ChannelStrategy,
        source::{NodeContext, Source, SourceContext},
        time::ArconTime,
    },
};
use kompact::prelude::*;
use std::cell::RefCell;

/// A message type that Source components in Arcon must implement
#[derive(Debug, PartialEq, Clone)]
pub enum SourceEvent {
    Epoch(Epoch),
    Watermark(ArconTime),
    Start,
}

#[derive(Debug, Clone, Copy)]
struct ProcessSource;
struct LoopbackPort;
impl Port for LoopbackPort {
    type Indication = Never;
    type Request = ProcessSource;
}

/// A [kompact] component to drive the execution of Arcon sources
#[derive(ComponentDefinition)]
pub struct SourceNode<S>
where
    S: Source,
{
    /// Component context
    ctx: ComponentContext<Self>,
    manager_port: RequiredPort<SourceManagerPort>,
    node_context: RefCell<NodeContext<S>>,
    loopback_send: RequiredPort<LoopbackPort>,
    loopback_receive: ProvidedPort<LoopbackPort>,
    source: RefCell<S>,
}

impl<S> SourceNode<S>
where
    S: Source,
{
    pub fn new(source: S, channel_strategy: ChannelStrategy<S::Data>) -> Self {
        Self {
            ctx: ComponentContext::uninitialised(),
            manager_port: RequiredPort::uninitialised(),
            node_context: RefCell::new(NodeContext {
                channel_strategy,
                watermark: 0,
                ended: false,
            }),
            loopback_send: RequiredPort::uninitialised(),
            loopback_receive: ProvidedPort::uninitialised(),
            source: RefCell::new(source),
        }
    }

    pub fn handle_source_event(&mut self, event: SourceEvent) {
        match event {
            SourceEvent::Epoch(epoch) => {
                self.node_context
                    .borrow_mut()
                    .channel_strategy
                    .add(ArconEvent::Epoch(epoch), self);
            }
            SourceEvent::Watermark(time) => {
                let wm = match time {
                    ArconTime::Event => Watermark::new(self.node_context.borrow().watermark),
                    ArconTime::Process => {
                        let system_time = crate::util::get_system_time();
                        Watermark::new(system_time)
                    }
                };

                // update internal watermark
                self.node_context.borrow_mut().watermark = wm.timestamp;

                // send watermark downstream
                self.node_context
                    .borrow_mut()
                    .channel_strategy
                    .add(ArconEvent::Watermark(wm), self);
            }
            SourceEvent::Start => {
                self.loopback_send.trigger(ProcessSource);
            }
        }
    }
}

impl<S> ComponentLifecycle for SourceNode<S>
where
    S: Source,
{
    fn on_start(&mut self) -> Handled {
        let shared = self.loopback_receive.share();
        self.loopback_send.connect(shared);
        Handled::Ok
    }
}

impl<S> Provide<LoopbackPort> for SourceNode<S>
where
    S: Source,
{
    fn handle(&mut self, _event: ProcessSource) -> Handled {
        self.source.borrow_mut().process_batch(SourceContext::new(
            self,
            &mut self.node_context.borrow_mut(),
        ));
        if self.node_context.borrow().ended {
            self.manager_port.trigger(SourceManagerEvent::End);
        } else {
            self.loopback_send.trigger(ProcessSource);
        }
        Handled::Ok
    }
}

impl<S> Require<LoopbackPort> for SourceNode<S>
where
    S: Source,
{
    fn handle(&mut self, _event: Never) -> Handled {
        unreachable!("Never type has no instance");
    }
}

impl<S> Require<SourceManagerPort> for SourceNode<S>
where
    S: Source,
{
    fn handle(&mut self, _event: Never) -> Handled {
        unreachable!("Never type has no instance");
    }
}

impl<S> Actor for SourceNode<S>
where
    S: Source,
{
    type Message = SourceEvent;

    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        self.handle_source_event(msg);
        Handled::Ok
    }
    fn receive_network(&mut self, _: NetMessage) -> Handled {
        Handled::Ok
    }
}
