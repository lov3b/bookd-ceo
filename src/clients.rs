use std::{
    borrow::Cow,
    cmp,
    fs::File,
    io::{self, BufReader},
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json as json;
use thiserror::Error;
use uuid::Uuid;

use crate::paths::get_paths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd)]
pub struct Client {
    pub id: Uuid,
    pub name: String,
    pub uses: usize,
    pub order: usize,
}

impl Ord for Client {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self.uses.cmp(&other.uses) {
            cmp::Ordering::Less => cmp::Ordering::Less,
            cmp::Ordering::Equal => self.order.cmp(&other.order),
            cmp::Ordering::Greater => cmp::Ordering::Greater,
        }
    }
}
pub struct ClientGroup {
    clients: Vec<Client>,
    next_index: usize,
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Serialization error: {0}")]
    Json(#[from] json::Error),
}

const FILE_NAME: &str = "clients.json";

impl ClientGroup {
    pub fn next(&mut self) -> &'_ Client {
        if self.next_index >= self.clients.len() {
            self.next_index = 0;
        }
        let ret = unsafe { self.clients.get(self.next_index).unwrap_unchecked() };
        self.next_index += 1;
        ret
    }

    pub fn load(predefined_path: Option<&Path>) -> Result<Self, LoadError> {
        let mut clients = Self::load_clients(predefined_path)?;
        clients.sort();
        clients
            .iter_mut()
            .enumerate()
            .for_each(|(index, client)| client.order = index);
        Ok(Self {
            clients,
            next_index: 0,
        })
    }

    pub fn add(&mut self, client: Client) {
        let index = client.order.max(self.clients.len());
        self.clients.insert(index, client);
        for i in (index + 1)..self.clients.len() {
            self.clients[i].order = i + 1;
        }
    }

    fn load_clients(predefined_path: Option<&Path>) -> Result<Vec<Client>, LoadError> {
        let paths = match predefined_path {
            Some(predefined) => {
                if predefined.is_dir() {
                    vec![Cow::Owned(predefined.join(FILE_NAME))]
                } else {
                    vec![Cow::Borrowed(predefined)]
                }
            }
            None => get_paths()
                .into_iter()
                .map(|base_dir| Cow::Owned(base_dir.join(FILE_NAME)))
                .collect::<Vec<_>>(),
        };

        match paths.iter().find(|p| p.exists()) {
            Some(path) => {
                let reader = BufReader::new(File::open(path)?);
                Ok(json::from_reader(reader)?)
            }
            None => {
                tracing::warn!("No existing file found. Creating new config");
                return Ok(Vec::new());
            }
        }
    }
}
