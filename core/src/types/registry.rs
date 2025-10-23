pub trait TypeCapabilities {
    fn supports_operation(&self, operation: &str) -> bool;
}
