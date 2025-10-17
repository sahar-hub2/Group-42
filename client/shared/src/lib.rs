// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GreetResponse {
    pub message: String,
}

pub fn greet(name: &str) -> GreetResponse {
    GreetResponse {
        message: format!(
            "Hello, {}! You've been greeted from the Rust backend!",
            name
        ),
    }
}
