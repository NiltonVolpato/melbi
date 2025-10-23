pub mod analyzer;
pub mod errors;
pub mod parser;
#[allow(dead_code)]
pub mod types;
#[allow(dead_code)]
pub mod values;

pub use types::Type;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
