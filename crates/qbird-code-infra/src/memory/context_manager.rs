pub struct ContextManager;

impl ContextManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}
