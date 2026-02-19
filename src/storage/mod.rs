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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tokens_used: Option<u64>,
    pub requests_made: Option<u64>,
    pub cost: Option<f64>,
}

impl QuotaSnapshot {
    pub fn from_quota_info(quota: &crate::providers::QuotaInfo) -> Self {
        Self {
            timestamp: quota.last_updated,
            tokens_used: quota.usage.tokens_used,
            requests_made: quota.usage.requests_made,
            cost: quota.usage.cost,
        }
    }

    pub fn has_changed_from(&self, other: &Self) -> bool {
        self.tokens_used != other.tokens_used
            || self.requests_made != other.requests_made
            || self.cost != other.cost
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaHistory {
    pub account_name: String,
    pub snapshots: Vec<QuotaSnapshot>,
}

const MAX_HISTORY_SIZE: usize = 100;

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

    fn history_path(&self) -> PathBuf {
        self.config_dir.join("quota_history.json")
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

    fn load_history(&self) -> Result<Vec<QuotaHistory>> {
        let path = self.history_path();

        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path).context("Failed to read quota history")?;

        let history: Vec<QuotaHistory> =
            serde_json::from_str(&content).context("Failed to parse quota history")?;

        Ok(history)
    }

    fn save_history(&self, history: &[QuotaHistory]) -> Result<()> {
        let content =
            serde_json::to_string_pretty(history).context("Failed to serialize quota history")?;

        fs::write(self.history_path(), content).context("Failed to write quota history")?;

        Ok(())
    }

    pub fn add_quota_snapshot(
        &self,
        account_name: &str,
        quota: &crate::providers::QuotaInfo,
    ) -> Result<bool> {
        let mut history = self.load_history()?;
        let new_snapshot = QuotaSnapshot::from_quota_info(quota);

        let account_history = history.iter_mut().find(|h| h.account_name == account_name);

        let changed = match account_history {
            Some(h) => {
                let changed = h
                    .snapshots
                    .last()
                    .map(|last| new_snapshot.has_changed_from(last))
                    .unwrap_or(true);

                if changed {
                    h.snapshots.push(new_snapshot);
                    // Keep only the last MAX_HISTORY_SIZE snapshots
                    if h.snapshots.len() > MAX_HISTORY_SIZE {
                        h.snapshots.remove(0);
                    }
                }
                changed
            }
            None => {
                // No history for this account yet, create new
                history.push(QuotaHistory {
                    account_name: account_name.to_string(),
                    snapshots: vec![new_snapshot],
                });
                true
            }
        };

        if changed {
            self.save_history(&history)?;
        }

        Ok(changed)
    }

    pub fn get_quota_history(&self, account_name: &str) -> Result<Vec<QuotaSnapshot>> {
        let history = self.load_history()?;

        Ok(history
            .into_iter()
            .find(|h| h.account_name == account_name)
            .map(|h| h.snapshots)
            .unwrap_or_default())
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

        // Remove quota history for this account
        let mut history = self.load_history()?;
        history.retain(|h| h.account_name != name);
        let _ = self.save_history(&history);

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

        let target = index
            .accounts
            .iter_mut()
            .find(|account| account.name == old_name)
            .context(format!("Account '{}' not found", old_name))?;

        let credentials = self.get_credentials(old_name)?;
        self.store_credentials(new_name, &credentials)?;
        self.delete_credentials(old_name)?;

        // Rename quota history
        let mut history = self.load_history()?;
        if let Some(h) = history.iter_mut().find(|h| h.account_name == old_name) {
            h.account_name = new_name.to_string();
            self.save_history(&history)?;
        }

        target.name = new_name.to_string();
        target.last_updated = chrono::Utc::now();

        self.save_index(&index)
    }

    pub fn save_accounts_order(&self, accounts: &[Account]) -> Result<()> {
        let index = AccountsIndex {
            accounts: accounts.to_vec(),
        };
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
