use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::schema::Message;

#[derive(Debug, Clone)]
pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new(data_dir: &Path, session_name: &str) -> Self {
        let path = data_dir
            .join("sessions")
            .join(format!("{}.jsonl", session_name));
        Self { path }
    }

    pub async fn load(&self) -> Result<Vec<Message>> {
        if !Path::new(&self.path).exists() {
            return Ok(Vec::new());
        }

        let file = tokio::fs::File::open(&self.path).await?;
        let mut reader = BufReader::new(file).lines();
        let mut out = Vec::new();

        while let Some(line) = reader.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let msg = serde_json::from_str::<Message>(&line)?;
            out.push(msg);
        }

        Ok(out)
    }

    pub async fn append(&self, msg: &Message) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        let line = serde_json::to_string(msg)?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }

    pub async fn rewrite(&self, messages: &[Message]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::File::create(&self.path).await?;
        for msg in messages {
            let line = serde_json::to_string(msg)?;
            file.write_all(line.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }
        Ok(())
    }

    pub async fn rename_to(&self, new_name: &str) -> Result<SessionStore> {
        let new_path = self
            .path
            .parent()
            .unwrap()
            .join(format!("{}.jsonl", new_name));
        tokio::fs::rename(&self.path, &new_path).await?;
        Ok(SessionStore { path: new_path })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::schema::Message;

    use super::SessionStore;

    #[tokio::test]
    async fn append_and_load() {
        let dir = tempdir().unwrap();
        let store = SessionStore::new(dir.path(), "test");
        store.append(&Message::user("hello")).await.unwrap();
        let loaded = store.load().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "hello");
    }
}
