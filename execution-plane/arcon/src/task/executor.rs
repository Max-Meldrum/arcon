use super::ArconTask;
use futures::{
    task::{waker_ref},
};
use kompact::prelude::*;
use std::sync::Arc;
use std::task::{Context, Poll};

struct ExecutorPort;

impl Port for ExecutorPort {
    type Indication = ();
    type Request = Arc<ArconTask>;
}

#[derive(ComponentDefinition)]
pub struct Executor {
    ctx: ComponentContext<Self>,
    executor_port: ProvidedPort<ExecutorPort, Self>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            ctx: ComponentContext::new(),
            executor_port: ProvidedPort::new(),
        }
    }

    fn run(&self, task: Arc<ArconTask>) {
        let future_slot = task.future.get();
        if let Some(mut future) = unsafe { (*future_slot).take() } {
            let waker = waker_ref(&task);
            let context = &mut Context::from_waker(&*waker);
            if let Poll::Pending = future.as_mut().poll(context) {
                // We're not done processing the future, so put it
                // back in its task to be run again in the future.
                let executor_slot = task.executor.get();
                unsafe {
                    // Set executor for future to be scheduled onto
                    *executor_slot = Some(self.actor_ref().hold().unwrap());
                    // Put back future for execution in the future
                    *future_slot = Some(future);
                };
            }
        }
    }
}

impl Provide<ControlPort> for Executor {
    fn handle(&mut self, _: ControlEvent) -> () {}
}

impl Provide<ExecutorPort> for Executor {
    fn handle(&mut self, task: Arc<ArconTask>) -> () {
        self.run(task);
    }
}

impl Actor for Executor {
    type Message = Arc<ArconTask>;

    fn receive_local(&mut self, task: Self::Message) {
        info!(self.ctx.log(), "{}", "Received a Task");
        self.run(task);
    }
    fn receive_network(&mut self, _: NetMessage) {}
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::UnsafeCell;
    use {
    futures::{
        future::{FutureExt, BoxFuture},
    }};

    #[test]
    fn simple_executor_test() {
        let system = KompactConfig::default().build().expect("KompactSystem");
        let executor = system.create_and_start(move || Executor::new());

        let s = async { 
            let l: i32 = 1 + 2;
            println!("run");
        };

        let actor_ref: ActorRefStrong<Arc<ArconTask>> =
            executor.actor_ref().hold().expect("Failed to fetch");

        let task = ArconTask::new(s);

        actor_ref.tell(task);
        std::thread::sleep(std::time::Duration::from_millis(2000));
    }

}
