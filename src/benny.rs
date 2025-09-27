use crate::player::Player;
use log::*;
use rand::Rng;
use std::sync::Arc;

pub struct Benny {
    player: Arc<Player>,
    file_path: Option<String>,
}
impl Benny {
    pub fn new(player: Arc<Player>, benny_abs_path: Option<String>) -> Self {
        Benny {
            player: player,
            file_path: benny_abs_path,
        }
    }

    pub fn toggle(&self) -> Result<(), anyhow::Error> {
        match self.file_path {
            Some(ref file_path) => {
                let seek = rand::rng().random_range(0..30000) as i64;
                info!("Toggle benny, seek {}ms", seek);
                self.player
                    .toggle_play(file_path.clone(), chrono::Duration::milliseconds(seek))
                    .map_err(|e| anyhow::anyhow!("{}", e))
            }
            None => {
                anyhow::bail!("Resources for Benny are not available")
            }
        }
    }
}
