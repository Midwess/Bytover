pub mod crafter;
pub mod devlog;
pub mod errors;
pub mod midwess;
pub mod value;

#[cfg(feature = "bindgen")]
uniffi::setup_scaffolding!();

pub fn main() {
    print!("Welcome to schema rs");
}
