use crate::player::Player;
use log::*;
use rand::Rng;
use std::sync::Arc;

pub struct Benny {
    player: Arc<Player>,
    file_path: String,
}
impl Benny {
    pub fn new(player: Arc<Player>, benny_abs_path: String) -> Self {
        Benny {
            player: player,
            file_path: benny_abs_path,
        }
    }

    pub fn toggle(&self) -> Result<(), anyhow::Error> {
        let seek = rand::rng().random_range(0..30000) as i64;
        info!("Toggle benny, seek {}ms", seek);
        self.player
            .toggle_play(self.file_path.clone(), chrono::Duration::milliseconds(seek))
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}
