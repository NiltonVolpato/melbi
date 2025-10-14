pub mod parser;

mod analyzer;
#[allow(dead_code)]
pub mod types;
#[allow(dead_code)]
mod values;

pub use types::manager::Type;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
