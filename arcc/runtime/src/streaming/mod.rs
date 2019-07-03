use crate::data::{ArconElement, ArconType};
use crate::prelude::{DeserializeOwned, Serialize};
use kompact::{ActorPath, ActorRef, ComponentDefinition, Port, Require, RequiredPort};
use std::cell::RefCell;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;

pub mod partitioner;
pub mod task;
pub mod window;

#[derive(Clone)]
pub struct ChannelPort<A: 'static + ArconType>(PhantomData<A>);

impl<A> Port for ChannelPort<A>
where
    A: 'static + ArconType,
{
    type Indication = ();
    type Request = ArconElement<A>;
}

#[derive(Clone)]
pub enum Channel<A, B, C>
where
    A: 'static + ArconType,
    B: Port<Request = ArconElement<A>> + 'static + Clone,
    C: ComponentDefinition + Sized + 'static + Require<B>,
{
    Local(ActorRef),
    Remote(ActorPath),
    Port(RequirePortRef<A, B, C>),
}

#[derive(Clone)]
pub struct RequirePortRef<A, B, C>(pub Rc<RefCell<RequiredPort<B, C>>>)
where
    A: 'static + ArconType,
    B: Port<Request = ArconElement<A>> + 'static + Clone,
    C: ComponentDefinition + Sized + 'static + Require<B>;

unsafe impl<A, B, C> Send for RequirePortRef<A, B, C>
where
    A: 'static + ArconType,
    B: Port<Request = ArconElement<A>> + 'static + Clone,
    C: ComponentDefinition + Sized + 'static + Require<B>,
{
}

unsafe impl<A, B, C> Sync for RequirePortRef<A, B, C>
where
    A: 'static + ArconType,
    B: Port<Request = ArconElement<A>> + 'static + Clone,
    C: ComponentDefinition + Sized + 'static + Require<B>,
{
}
