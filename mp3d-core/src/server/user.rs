use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub password_hash: String,
    pub created_at: u64,
    pub last_login: u64,
}

pub struct UserDatabase {
    pub users: HashMap<String, User>,
    pub file_path: PathBuf,
}

impl UserDatabase {
    pub fn load(file_path: PathBuf) -> Self {
        if let Ok(data) = std::fs::read(&file_path) {
            if let Ok(users) = serde_json::from_slice(&data) {
                return Self { users, file_path };
            }
        }
        Self {
            users: HashMap::new(),
            file_path,
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let data = serde_json::to_vec(&self.users)?;
        std::fs::write(&self.file_path, data)
    }

    pub fn register(&mut self, username: String, password: String) -> Result<(), String> {
        if self.users.contains_key(&username) {
            return Err("Username already exists".to_string());
        }
        let user = User {
            password_hash: bcrypt::hash(password, bcrypt::DEFAULT_COST)
                .map_err(|e| format!("Failed to hash password: {}", e))?,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_login: 0,
        };
        self.users.insert(username, user);
        Ok(())
    }

    pub fn login(&mut self, username: &str, password: &str) -> Result<(), String> {
        if let Some(user) = self.users.get_mut(username) {
            if bcrypt::verify(password, &user.password_hash)
                .map_err(|e| format!("Failed to verify password: {}", e))?
            {
                user.last_login = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                Ok(())
            } else {
                Err("Incorrect password".to_string())
            }
        } else {
            Err("Username not found".to_string())
        }
    }

    pub fn login_or_register(&mut self, username: String, password: String) -> Result<(), String> {
        if self.users.contains_key(&username) {
            self.login(&username, &password)
        } else {
            self.register(username, password)
        }
    }
}
