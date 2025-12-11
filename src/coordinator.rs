// File: src/coordinator.rs
use crate::booking::Booking;
use crate::clients::{Client, ClientGroup};
use serde::Serialize;
use std::collections::HashMap;
use tokio::sync::broadcast;
use uuid::Uuid;

const CAPACITY: usize = 100;

#[derive(Debug, Clone, Serialize)]
pub struct Assignment {
    pub booking: Booking,
    pub assigned_client: Client,
}

pub struct Coordinator {
    client_group: ClientGroup,
    assignments: HashMap<Uuid, Vec<Booking>>,
    tx: broadcast::Sender<Assignment>,
}

impl Coordinator {
    pub fn new(client_group: ClientGroup) -> Self {
        let (tx, _) = broadcast::channel(CAPACITY);
        Self {
            client_group,
            assignments: HashMap::new(),
            tx,
        }
    }

    pub async fn schedule_and_broadcast(&mut self, booking: Booking) -> Assignment {
        let client_arc = self.client_group.next();

        let assignment = {
            let guard = client_arc.lock().await;
            self.assignments
                .entry(guard.id)
                .or_insert_with(Vec::new)
                .push(booking.clone());
            Assignment {
                booking,
                assigned_client: guard.clone(),
            }
        };

        match self.tx.send(assignment.clone()) {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("Failed to send assignment: {:?}", e)
            }
        }

        assignment
    }

    /// Create a new receiver for a client
    pub fn subscribe(&self) -> broadcast::Receiver<Assignment> {
        self.tx.subscribe()
    }

    pub fn get_status_count(&self) -> usize {
        self.assignments.values().map(Vec::len).sum()
    }
}
