use anyhow::{bail, Context, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccountsIndex {
    accounts: Vec<Account>,
}

pub struct SecureStorage {
    config_dir: PathBuf,
}

impl SecureStorage {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("tokstat");

        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        Ok(Self { config_dir })
    }

    fn index_path(&self) -> PathBuf {
        self.config_dir.join("accounts.json")
    }

    fn load_index(&self) -> Result<AccountsIndex> {
        let path = self.index_path();

        if !path.exists() {
            return Ok(AccountsIndex {
                accounts: Vec::new(),
            });
        }

        let content = fs::read_to_string(&path).context("Failed to read accounts index")?;

        let index: AccountsIndex =
            serde_json::from_str(&content).context("Failed to parse accounts index")?;

        Ok(index)
    }

    fn save_index(&self, index: &AccountsIndex) -> Result<()> {
        let content =
            serde_json::to_string_pretty(index).context("Failed to serialize accounts index")?;

        fs::write(self.index_path(), content).context("Failed to write accounts index")?;

        Ok(())
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let index = self.load_index()?;
        Ok(index.accounts)
    }

    pub fn get_account(&self, name: &str) -> Result<Account> {
        let index = self.load_index()?;

        index
            .accounts
            .into_iter()
            .find(|acc| acc.name == name)
            .context(format!("Account '{}' not found", name))
    }

    pub fn save_account(&self, account: Account) -> Result<()> {
        let mut index = self.load_index()?;

        // Remove existing account with same name
        index.accounts.retain(|acc| acc.name != account.name);

        // Add new account
        index.accounts.push(account);

        self.save_index(&index)
    }

    pub fn remove_account(&self, name: &str) -> Result<()> {
        let mut index = self.load_index()?;

        let len_before = index.accounts.len();
        index.accounts.retain(|acc| acc.name != name);

        if index.accounts.len() == len_before {
            anyhow::bail!("Account '{}' not found", name);
        }

        // Also remove credentials from keyring
        self.delete_credentials(name)?;

        self.save_index(&index)
    }

    pub fn store_credentials(&self, account_name: &str, credentials: &str) -> Result<()> {
        let entry =
            Entry::new("tokstat", account_name).context("Failed to create keyring entry")?;

        entry
            .set_password(credentials)
            .context("Failed to store credentials in keyring")?;

        Ok(())
    }

    pub fn get_credentials(&self, account_name: &str) -> Result<String> {
        let entry =
            Entry::new("tokstat", account_name).context("Failed to create keyring entry")?;

        entry
            .get_password()
            .context("Failed to retrieve credentials from keyring")
    }

    pub fn delete_credentials(&self, account_name: &str) -> Result<()> {
        let entry =
            Entry::new("tokstat", account_name).context("Failed to create keyring entry")?;

        // Ignore error if entry doesn't exist
        let _ = entry.delete_password();

        Ok(())
    }

    pub fn rename_account(&self, old_name: &str, new_name: &str) -> Result<()> {
        if old_name == new_name {
            return Ok(());
        }

        let mut index = self.load_index()?;

        if index
            .accounts
            .iter()
            .any(|account| account.name == new_name)
        {
            bail!("An account named '{}' already exists", new_name);
        }

        let mut target = index
            .accounts
            .iter_mut()
            .find(|account| account.name == old_name)
            .context(format!("Account '{}' not found", old_name))?;

        let credentials = self.get_credentials(old_name)?;
        self.store_credentials(new_name, &credentials)?;
        self.delete_credentials(old_name)?;

        target.name = new_name.to_string();
        target.last_updated = chrono::Utc::now();

        self.save_index(&index)
    }
}

// Add missing dirs dependency
mod dirs {
    use std::env;
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg_config));
        }

        if let Ok(home) = env::var("HOME") {
            return Some(PathBuf::from(home).join(".config"));
        }

        None
    }
}
