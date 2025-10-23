pub mod analyzer;
pub mod errors;
pub mod parser;
pub mod types;
pub mod values;
pub use types::Type;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
