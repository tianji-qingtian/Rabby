//! Secret store boundary. Keychain and encrypted vault backends land in M2.

pub trait SecretStore {
    fn get(&self, key: &str) -> anyhow::Result<Option<String>>;
    fn set(&mut self, key: &str, value: &str) -> anyhow::Result<()>;
    fn delete(&mut self, key: &str) -> anyhow::Result<()>;
}
