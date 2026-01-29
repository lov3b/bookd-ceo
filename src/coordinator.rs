use crate::booking::Booking;
use crate::clients::{Client, ClientGroup};
use serde::Serialize;
use std::collections::HashMap;
use tokio::sync::broadcast;
use uuid::Uuid;

const CAPACITY: usize = 100;

#[derive(Debug, Clone, Serialize)]
pub enum BroadcastEvent {
    Assigned(Assignment),
    Cancelled(Uuid),
}

#[derive(Debug, Clone, Serialize)]
pub struct Assignment {
    pub booking: Booking,
    pub assigned_client: Client,
}

pub struct Coordinator {
    client_group: ClientGroup,
    assignments: HashMap<Uuid, Vec<Booking>>,
    tx: broadcast::Sender<BroadcastEvent>,
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

    pub async fn client_identified(&self, id: Uuid) -> bool {
        self.client_group.contains(id).await
    }

    pub async fn get_assignments(&self, id: &Uuid) -> Vec<Assignment> {
        let mut all = Vec::new();
        let (_, bookings) = match self
            .assignments
            .iter()
            .find(|(client_id, _)| *client_id == id)
        {
            Some(ret) => ret,
            None => {
                return all;
            }
        };

        if let Some(client) = self.client_group.get_copy(id).await {
            all.extend(bookings.into_iter().map(|booking| Assignment {
                booking: booking.clone(),
                assigned_client: client.clone(),
            }));
        }

        all
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

        // Wrap in the event enum
        let event = BroadcastEvent::Assigned(assignment.clone());

        if let Err(e) = self.tx.send(event) {
            tracing::warn!("Failed to broadcast assignment: {:?}", e);
        }

        assignment
    }

    pub fn cancel_and_broadcast(&mut self, booking_id: Uuid) -> bool {
        let mut found = false;

        for bookings in self.assignments.values_mut() {
            if let Some(pos) = bookings.iter().position(|b| b.id == booking_id) {
                bookings.remove(pos);
                found = true;
                break;
            }
        }

        if found {
            let event = BroadcastEvent::Cancelled(booking_id);
            if let Err(e) = self.tx.send(event) {
                tracing::warn!("Failed to broadcast cancellation: {:?}", e);
            }
        }

        found
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BroadcastEvent> {
        self.tx.subscribe()
    }

    pub fn get_status_count(&self) -> usize {
        self.assignments.values().map(Vec::len).sum()
    }
}
