const MEMORY_USAGE: &str = "memory.usage_in_bytes";
const MEMORY_LIMIT: &str = "memory.limit_in_bytes";

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Memory {}
