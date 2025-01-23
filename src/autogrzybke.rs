use anyhow::Context;
use rand::seq::SliceRandom;
use rand::RngCore;
use std::iter;
use std::ops::Add;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use crate::resource_catalogue::{list_files_recursive, ResourceCatalogue};

pub fn canoncialize_resources_path(resources_path: &str) -> String {
    std::fs::canonicalize(resources_path)
        .context(format!(
            "Failed to use {resources_path} as autogrzybke resources path"
        ))
        .unwrap()
        .as_os_str()
        .to_str()
        .unwrap()
        .to_string()
}

struct AutogrzybkeImpl {
    resources: ResourceCatalogue,
    resources_path: String,
    recent_usage_time_window: Duration,
    recent_usage_timestamps: Vec<SystemTime>,
    last_missing_list: Vec<String>,
    prefix_chance_percent: u64,
    suffix_chance_percent: u64,
}
impl AutogrzybkeImpl {
    fn new(resources_path: String, prefix_chance_percent: u64, suffix_chance_percent: u64) -> Self {
        AutogrzybkeImpl {
            resources: ResourceCatalogue::try_from_dir_path(&Path::new(&resources_path)).unwrap(),
            resources_path: resources_path,
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

    fn generate_playlist(&mut self, missing: Vec<String>) -> Vec<String> {
        if missing.is_empty() {
            self.generate_ready_playlist()
        } else {
            self.generate_waiting_playlist(missing)
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

    fn generate_waiting_playlist(&mut self, missing: Vec<String>) -> Vec<String> {
        self.last_missing_list = missing.clone();
        self.last_missing_list.sort_unstable();
        let prefix_chance_percent = self.prefix_chance_percent;
        let suffix_chance_percent = self.suffix_chance_percent;
        let mut rng = rand::rng();
        let mut missing = missing
            .iter()
            .map(|nick| {
                let mut shoutout = Vec::new();
                if rng.next_u64() % 100 <= prefix_chance_percent {
                    shoutout.push("silence".to_string());
                    shoutout.push("prefix".to_string());
                }
                shoutout.push(nick.clone());
                if rng.next_u64() % 100 <= suffix_chance_percent {
                    shoutout.push("suffix".to_string());
                    shoutout.push("silence".to_string());
                }
                shoutout
            })
            .chain(
                iter::repeat(vec!["kurwa".to_string()])
                    .take(0.max((self.get_usage_count() - 1) / 2 - 1) as usize),
            )
            .collect::<Vec<Vec<String>>>();
        missing.shuffle(&mut rng);
        let mut words: Vec<String> = missing.into_iter().flatten().collect();
        words.push("lobby".to_string());
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
    fn list_resources(&self) -> Vec<String> {
        match list_files_recursive(&self.resources_path) {
            Ok(mut list) => {
                list.sort();
                list.iter()
                    .map(|name| name.to_str().unwrap().to_string())
                    .filter(|name| name.ends_with(".mp3"))
                    .collect()
            }
            Err(e) => vec![e.to_string()],
        }
    }
}

pub struct Autogrzybke {
    autogrzybke_impl: Mutex<AutogrzybkeImpl>,
}
impl Autogrzybke {
    pub fn new(
        resources_abs_path: String,
        prefix_chance_percent: u64,
        suffix_chance_percent: u64,
    ) -> Self {
        Autogrzybke {
            autogrzybke_impl: Mutex::new(AutogrzybkeImpl::new(
                resources_abs_path,
                prefix_chance_percent,
                suffix_chance_percent,
            )),
        }
    }
    pub fn generate_playlist(&self, missing: Vec<String>) -> Vec<String> {
        self.autogrzybke_impl
            .lock()
            .unwrap()
            .generate_playlist(missing)
    }

    pub fn get_last_missing(&self) -> Vec<String> {
        self.autogrzybke_impl.lock().unwrap().get_last_missing()
    }

    pub fn list_resources(&self) -> Vec<String> {
        self.autogrzybke_impl.lock().unwrap().list_resources()
    }
}
