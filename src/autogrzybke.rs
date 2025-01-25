use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Deserializer};
use std::iter;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::resource_catalogue::ResourceCatalogue;

struct AutogrzybkeImpl {
    resources: Arc<ResourceCatalogue>,
    recent_usage_time_window: Duration,
    recent_usage_timestamps: Vec<SystemTime>,
    last_missing_list: Vec<String>,
    prefix_chance_percent: u64,
    suffix_chance_percent: u64,
}
impl AutogrzybkeImpl {
    fn new(
        resources: Arc<ResourceCatalogue>,
        prefix_chance_percent: u64,
        suffix_chance_percent: u64,
    ) -> Self {
        AutogrzybkeImpl {
            resources: resources,
            recent_usage_time_window: Duration::from_secs(60 * 15),
            recent_usage_timestamps: Vec::new(),
            last_missing_list: Vec::new(),
            prefix_chance_percent: prefix_chance_percent,
            suffix_chance_percent: suffix_chance_percent,
        }
    }

    fn get_usage_count(&mut self) -> i64 {
        let now = SystemTime::now();
        self.recent_usage_timestamps.push(now);
        self.recent_usage_timestamps
            .retain(|timestamp| timestamp.add(self.recent_usage_time_window) > now);
        self.recent_usage_timestamps.len() as i64
    }

    fn generate_playlist(&mut self, req: AutogrzybkeRequest) -> Vec<String> {
        if req.missing.is_empty() {
            self.generate_ready_playlist()
        } else {
            self.generate_waiting_playlist(req)
        }
    }

    fn generate_ready_playlist(&mut self) -> Vec<String> {
        self.recent_usage_timestamps.clear();
        self.last_missing_list.clear();
        ["everyone", "ready"]
            .iter()
            .flat_map(|sample| self.resources.random_sample(sample))
            .collect()
    }

    fn generate_waiting_playlist(&mut self, req: AutogrzybkeRequest) -> Vec<String> {
        self.last_missing_list = req.missing.clone();
        self.last_missing_list.sort_unstable();
        let prefix_chance_percent = self.prefix_chance_percent;
        let suffix_chance_percent = self.suffix_chance_percent;
        let mut rng = rand::rng();
        let mut missing = req
            .missing
            .iter()
            .map(|nick| {
                let mut shoutout = Vec::new();
                let shall_add_prefix =
                    rng.random_range(0..100) < prefix_chance_percent && !req.skip_prefix;
                let shall_add_suffix =
                    rng.random_range(0..100) < suffix_chance_percent && !req.skip_suffix;

                if shall_add_prefix || shall_add_suffix {
                    shoutout.push("silence".to_string());
                }
                if shall_add_prefix {
                    shoutout.push("prefix".to_string());
                }
                shoutout.push(nick.clone());
                if shall_add_suffix {
                    shoutout.push("suffix".to_string());
                }
                if shall_add_prefix || shall_add_suffix {
                    shoutout.push("silence".to_string());
                }
                shoutout
            })
            .chain(
                iter::repeat(vec!["kurwa".to_string()]).take(if !req.skip_interlude {
                    0.max((self.get_usage_count() - 1) / 2 - 1) as usize
                } else {
                    0
                }),
            )
            .collect::<Vec<Vec<String>>>();
        missing.shuffle(&mut rng);
        let mut words: Vec<String> = missing.into_iter().flatten().collect();
        words.extend(if !req.skip_lobby {
            Some("lobby".to_string())
        } else {
            None
        });
        words
            .iter()
            .flat_map(|sample| {
                self.resources
                    .random_sample(sample)
                    .or_else(|| self.resources.random_sample("unknown"))
            })
            .collect()
    }

    fn get_last_missing(&self) -> Vec<String> {
        self.last_missing_list.clone()
    }
}

pub struct Autogrzybke {
    autogrzybke_impl: Mutex<AutogrzybkeImpl>,
}
impl Autogrzybke {
    pub fn new(
        resources: Arc<ResourceCatalogue>,
        prefix_chance_percent: u64,
        suffix_chance_percent: u64,
    ) -> Self {
        Autogrzybke {
            autogrzybke_impl: Mutex::new(AutogrzybkeImpl::new(
                resources,
                prefix_chance_percent,
                suffix_chance_percent,
            )),
        }
    }
    pub fn generate_playlist(&self, req: AutogrzybkeRequest) -> Vec<String> {
        self.autogrzybke_impl.lock().unwrap().generate_playlist(req)
    }

    pub fn get_last_missing(&self) -> Vec<String> {
        self.autogrzybke_impl.lock().unwrap().get_last_missing()
    }
}

#[derive(Deserialize)]
pub struct AutogrzybkeRequest {
    #[serde(deserialize_with = "deserialize_whitespace_separated")]
    pub missing: Vec<String>,

    // Optional customization flags
    #[serde(default)]
    pub skip_lobby: bool,
    #[serde(default)]
    pub skip_prefix: bool,
    #[serde(default)]
    pub skip_suffix: bool,
    #[serde(default)]
    pub skip_interlude: bool,
}

fn deserialize_whitespace_separated<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    Ok(text.split_whitespace().map(String::from).collect())
}
