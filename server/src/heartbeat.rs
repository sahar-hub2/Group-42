// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

use crate::AppState;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::info;

/// Spawns a background task that periodically checks for users who missed 3 heartbeats (45s)
pub fn spawn_heartbeat_cleanup(app_state: Arc<Mutex<AppState>>) {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            let mut state = app_state.lock().unwrap();
            let now = Instant::now();
            let mut to_remove = vec![];
            for (id, last) in state.user_heartbeat.iter() {
                if now.duration_since(*last) > Duration::from_secs(45) {
                    to_remove.push(id.clone());
                }
            }
            for id in to_remove {
                state.local_users.lock().unwrap().remove(&id);
                state.user_locations.lock().unwrap().remove(&id);
                state.user_heartbeat.remove(&id);
                info!(user_id = %id.to_string(), "User removed due to missed heartbeats");
            }
        }
    });
}
