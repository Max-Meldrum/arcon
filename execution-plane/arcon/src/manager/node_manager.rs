// Copyright (c) 2020, KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    data::ArconType,
    manager::state_manager::StateManager,
    metrics::node_metrics::NodeMetrics,
    prelude::NodeID,
    stream::{channel::strategy::ChannelStrategy, node::Node},
    util::SafelySendableFn,
};
use fxhash::FxHashMap;
use kompact::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum NodeEvent {
    Metrics(NodeID, NodeMetrics),
    Update,
    Reconfig,
}

pub struct NodeManagerPort {}
impl Port for NodeManagerPort {
    type Indication = ();
    type Request = NodeEvent;
}

/// A [kompact] component responsible for coordinating a set of Arcon nodes
///
/// The following illustrates the role of a NodeManager in the context of an ArconPipeline
///
/// ```text
///                 ArconPipeline
///                /             \
///         NodeManager <----> NodeManager
///             |                  |
///          MapNode1  ------> WindowNode1
///             |                  |
///          MapNode2  ------> WindowNode2
///
/// ```
#[derive(ComponentDefinition)]
pub struct NodeManager<IN, OUT>
where
    IN: ArconType,
    OUT: ArconType,
{
    /// Component Context
    ctx: ComponentContext<Self>,
    /// A text description of the operating NodeManager
    ///
    /// e.g., window_sliding_avg_price
    node_description: String,
    /// Reference to the state manager
    state_manager: Arc<Component<StateManager>>,
    /// Port for incoming local events
    manager_port: ProvidedPort<NodeManagerPort, Self>,
    /// Current Node parallelism
    node_parallelism: usize,
    /// Max Node parallelism
    max_node_parallelism: usize,
    /// Current Node IDs that are connected to nodes on this manager
    in_channels: Vec<NodeID>,
    /// ChannelStrategy used by the nodes on this manager
    channel_strategy: ChannelStrategy<OUT>,
    /// Monotonically increasing Node ID index
    node_index: u32,
    /// Nodes this manager controls
    nodes: FxHashMap<NodeID, Arc<Component<Node<IN, OUT>>>>,
    /// Metrics per Node
    node_metrics: FxHashMap<NodeID, NodeMetrics>,
    /// Port reference to the previous NodeManager in the pipeline stage
    ///
    /// It is defined as an Option as source components won't have any prev_manager
    prev_manager: Option<RequiredRef<NodeManagerPort>>,
    /// Port reference to the next NodeManager in the pipeline stage
    ///
    /// It is defined as an Option as sink components won't have any next_manager
    next_manager: Option<RequiredRef<NodeManagerPort>>,
    /// Function to create a Node on this NodeManager
    node_fn:
        &'static dyn SafelySendableFn(NodeID, Vec<NodeID>, ChannelStrategy<OUT>) -> Node<IN, OUT>,
}

impl<IN, OUT> NodeManager<IN, OUT>
where
    IN: ArconType,
    OUT: ArconType,
{
    pub fn new(
        node_description: String,
        node_fn: &'static dyn SafelySendableFn(
            NodeID,
            Vec<NodeID>,
            ChannelStrategy<OUT>,
        ) -> Node<IN, OUT>,
        channel_strategy: ChannelStrategy<OUT>,
        in_channels: Vec<NodeID>,
        node_comps: Vec<Arc<Component<Node<IN, OUT>>>>,
        state_manager: Arc<Component<StateManager>>,
        prev_manager: Option<RequiredRef<NodeManagerPort>>,
        next_manager: Option<RequiredRef<NodeManagerPort>>,
    ) -> NodeManager<IN, OUT> {
        let total_nodes = node_comps.len() as u32;
        let mut nodes_map = FxHashMap::default();
        for (i, node) in node_comps.into_iter().enumerate() {
            let node_id = NodeID::new(i as u32);
            nodes_map.insert(node_id, node);
        }
        NodeManager {
            ctx: ComponentContext::new(),
            node_description,
            state_manager,
            manager_port: ProvidedPort::new(),
            node_parallelism: total_nodes as usize,
            max_node_parallelism: (num_cpus::get() * 2) as usize,
            in_channels,
            channel_strategy,
            nodes: nodes_map,
            node_metrics: FxHashMap::default(),
            node_index: total_nodes,
            prev_manager,
            next_manager,
            node_fn,
        }
    }
}

impl<IN, OUT> Provide<ControlPort> for NodeManager<IN, OUT>
where
    IN: ArconType,
    OUT: ArconType,
{
    fn handle(&mut self, event: ControlEvent) {
        match event {
            ControlEvent::Start => {
                info!(
                    self.ctx.log(),
                    "Started NodeManager for {}", self.node_description
                );

                let mut manager_port = &mut self.manager_port;
                // For each node, connect its NodeManagerPort to NodeManager
                for (_, node) in &self.nodes {
                    let mut node_port = &node.on_definition(|cd| {
                        let mut p = &mut cd.node_manager_port;
                        biconnect_ports::<NodeManagerPort, _, _>(manager_port, p);
                    });
                }
            }
            ControlEvent::Kill => {}
            ControlEvent::Stop => {}
        }
    }
}

impl<IN, OUT> Provide<NodeManagerPort> for NodeManager<IN, OUT>
where
    IN: ArconType,
    OUT: ArconType,
{
    fn handle(&mut self, event: NodeEvent) {
        debug!(self.ctx.log(), "Got Event {:?}", event);
        match event {
            NodeEvent::Metrics(id, metrics) => {
                self.node_metrics.insert(id, metrics);
            }
            _ => {}
        }
    }
}

impl<IN, OUT> Require<NodeManagerPort> for NodeManager<IN, OUT>
where
    IN: ArconType,
    OUT: ArconType,
{
    fn handle(&mut self, event: ()) {}
}

impl<IN, OUT> Actor for NodeManager<IN, OUT>
where
    IN: ArconType,
    OUT: ArconType,
{
    type Message = NodeEvent;
    fn receive_local(&mut self, _: Self::Message) {}
    fn receive_network(&mut self, _: NetMessage) {}
}
