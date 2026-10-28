#![doc = include_str!("../README.md")]

/// A greeting message.
pub struct Greeting {
    /// The name to greet.
    pub name: String,
}

impl Greeting {
    /// Create a greeting for the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Render the greeting as a string.
    pub fn render(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

/// Produce a default greeting.
pub fn default_greeting() -> Greeting {
    Greeting::new("world")
}
