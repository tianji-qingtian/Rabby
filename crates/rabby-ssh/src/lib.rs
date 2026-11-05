//! SSH transport skeleton. russh integration lands in M2.

#[derive(Debug, Clone)]
pub struct SshProfile {
    pub host: String,
    pub port: u16,
    pub user: String,
}

impl Default for SshProfile {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            user: String::new(),
        }
    }
}
