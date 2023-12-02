mod local;
mod websocket;

use std::collections::HashMap;
pub use local::LocalTouchService;
pub use websocket::WebsocketTouchService;

pub struct PointerInfos {
    // pointer id -> relative location
    pub map: HashMap<u32, (i32, i32)>,
    // radius
    pub radius: u32,
}

// TouchService trait.
// Note: Structs implementing this trait should do initialization in their new() function.
pub trait TouchService {
    // Returns a pointer to map of pointer id -> **relative location**
    fn get_info(&self) -> PointerInfos;

    // Perform main poll logic here.
    fn main_cycle(&self);
}