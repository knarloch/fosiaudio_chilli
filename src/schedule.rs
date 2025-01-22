use crate::autogrzybke::parse_resources_variant_count_from_path;
use crate::player::Player;
use anyhow::Context;
use chrono::{DateTime, Local};
use log::*;
use rand::Rng;
use std::sync::{Arc, Mutex};

struct SchedulerImpl {
    player: Arc<Player>,
    resources_path: String,
    resources_variant_count: u64,
    schedule: Vec<DateTime<Local>>,
}

fn parse_and_filter_schedule(text: &str) -> Result<Vec<DateTime<Local>>, anyhow::Error> {
    let now = Local::now();
    let mut schedule: Vec<DateTime<Local>> =
        serde_yaml::from_str(text).context(format!("Parse schedule from \"{text}\""))?;
    schedule.retain(|tp| *tp >= now);
    schedule.sort_unstable();
    info!("now: {:?}", now);
    info!("Schedule: {:?}", schedule);
    Ok(schedule)
}

pub const SCHEDULE_DEFAULT: &str = include_str!("schedule_default.yaml");
impl SchedulerImpl {
    fn new(player: Arc<Player>, resources_path: String) -> Result<Self, anyhow::Error> {
        Ok(SchedulerImpl {
            player: player,
            resources_variant_count: parse_resources_variant_count_from_path(
                resources_path.as_str(),
            )?,
            resources_path: resources_path,
            schedule: parse_and_filter_schedule(SCHEDULE_DEFAULT)?,
        })
    }

    fn get_serialized_schedule(&self) -> Result<String, anyhow::Error> {
        if self.schedule.is_empty() {
            Ok("nie idziemy :(".to_string())
        } else {
            Ok(serde_yaml::to_string(&self.schedule).context("Serialize current schedule")?)
        }
    }
}

pub struct Scheduler {
    schedule_impl: Mutex<SchedulerImpl>,
}
impl Scheduler {
    pub fn new(player: Arc<Player>, resources_path: String) -> Result<Self, anyhow::Error> {
        Ok(Scheduler {
            schedule_impl: Mutex::new(SchedulerImpl::new(player, resources_path)?),
        })
    }

    pub fn get_serialized_schedule(&self) -> Result<String, anyhow::Error> {
        self.schedule_impl.lock().unwrap().get_serialized_schedule()
    }

    pub fn set_schedule(&self, text: &str) -> Result<(), anyhow::Error> {
        let schedule =
            parse_and_filter_schedule(text).context(format!("Parse schedule from \"{text}\""))?;
        self.schedule_impl.lock().unwrap().schedule = schedule;
        Ok(())
    }

    pub async fn run_schedule(&self) -> () {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        loop {
            interval.tick().await;
            let now = Local::now();
            {
                let mut schedule_impl = self.schedule_impl.lock().unwrap();
                if let Some(closest_event) = schedule_impl.schedule.first() {
                    if *closest_event <= now {
                        info!(
                            "Now: {:?}, closest_event: {:?}. Triggering event.",
                            now, closest_event
                        );
                        let mut rng = rand::rng();
                        let playlist = ["idziemy_na_jednego"]
                            .iter()
                            .map(|sample| {
                                format!(
                                    "{0}/{sample}{1}.mp3",
                                    schedule_impl.resources_path,
                                    rng.random::<u64>() % (schedule_impl.resources_variant_count)
                                        + 1
                                )
                                .to_ascii_lowercase()
                            })
                            .collect();
                        schedule_impl
                            .player
                            .play_local_playlist(playlist)
                            .context("play from schedule")
                            .unwrap_or_else(|e| log::error!("Failed to play schedule: {e}"));
                        schedule_impl.schedule.remove(0);
                        info!("Next closest_event: {:?}", schedule_impl.schedule.first());
                    }
                }
            }
        }
    }
}
