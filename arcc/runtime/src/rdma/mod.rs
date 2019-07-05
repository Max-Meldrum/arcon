pub mod receiver;

pub struct RdmaContext {
    infinity_ctx: infinity::core::Context,
}

impl RdmaContext {
    pub fn new() -> RdmaContext {
        RdmaContext {
            infinity_ctx: infinity::core::Context::new(0, 1),
        }
    }
}

unsafe impl Sync for RdmaContext {}
unsafe impl Send for RdmaContext {}
