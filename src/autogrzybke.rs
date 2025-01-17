use anyhow::Context;
use rand::seq::SliceRandom;
use rand::Rng;
use std::fs::{canonicalize, read_to_string};
use std::{fs, iter};
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

struct AutogrzybkeImpl {
    resources_path: String,
    resources_variant_count: u64,
    recent_usage_time_window: Duration,
    recent_usage_timestamps: Vec<SystemTime>,
    last_missing_list: Vec<String>,
}
impl AutogrzybkeImpl {
    fn parse_resources_variant_count_from_path(path: &str) -> Result<u64, anyhow::Error> {
        let sample_sets_count_txt = Path::new(path).join("sample_sets_count.txt");
        let sample_sets_count_txt = sample_sets_count_txt.as_path();
        let lines: u64 = read_to_string(sample_sets_count_txt)
            .context(format!(
                "Failed to read {}",
                sample_sets_count_txt.display()
            ))?
            .trim()
            .parse()
            .context(format!(
                "Failed to parse u64 from {}",
                sample_sets_count_txt.display()
            ))?;
        Ok(lines)
    }

    fn new(resources_path: &str) -> Self {
        AutogrzybkeImpl {
            resources_path: std::fs::canonicalize(resources_path)
                .context(format!(
                    "Failed to use {resources_path} as autogrzybke resources path"
                ))
                .unwrap()
                .as_os_str()
                .to_str()
                .unwrap()
                .to_string(),
            resources_variant_count: Self::parse_resources_variant_count_from_path(resources_path)
                .unwrap(),
            recent_usage_time_window: Duration::from_secs(60 * 15),
            recent_usage_timestamps: Vec::new(),
            last_missing_list: Vec::new(),
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
        let mut rng = rand::rng();
        ["everyone", "ready"]
            .iter()
            .map(|sample| {
                format!(
                    "{0}/{sample}{1}.mp3",
                    self.resources_path,
                    rng.random::<u64>() % (self.resources_variant_count) + 1
                )
                .to_ascii_lowercase()
            })
            .collect()
    }

    fn generate_waiting_playlist(&mut self, mut missing: Vec<String>) -> Vec<String> {
        self.last_missing_list = missing.clone();
        self.last_missing_list.sort_unstable();
        let mut rng = rand::rng();
        missing.extend(
            iter::repeat("kurwa".to_string())
                .take(0.max((self.get_usage_count() - 1) / 2 - 1) as usize),
        );
        missing.shuffle(&mut rng);
        missing.push("lobby".to_string());
        missing
            .iter()
            .map(|nickname| {
                let mut filepath = format!(
                    "{0}/{nickname}{1}.mp3",
                    self.resources_path,
                    rng.random::<u64>() % (self.resources_variant_count) + 1
                )
                .to_ascii_lowercase();
                while canonicalize(filepath.clone()).is_err() {
                    filepath = format!(
                        "{0}/unknown{1}.mp3",
                        self.resources_path,
                        rng.random::<u64>() % (self.resources_variant_count) + 1
                    )
                }
                filepath
            })
            .collect()
    }

    fn get_last_missing(&self) -> Vec<String> {
        self.last_missing_list.clone()
    }
    fn list_resources(&self) -> Vec<String> {
        let mut list: Vec<PathBuf> = Vec::new();

        match list_files_recursive(Path::new(&self.resources_path),  &mut list) {
            Ok(()) =>{
                list.sort();
                list.iter().map(|name| name.to_str().unwrap().to_string()).filter(|name| name.ends_with(".mp3")).collect()
            }
            Err(e) => vec![e.to_string()],
        }
    }
}

fn list_files_recursive(dir: &Path, list: &mut Vec<PathBuf>) -> Result<(), anyhow::Error> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).context(format!("Failed to read_dir {dir:?}"))? {
            let entry = entry.context(format!("Failed to get entry from {dir:?}"))?;
            let path = entry.path();
            if path.is_dir() {
                list_files_recursive(&path, list)?;
            } else {
               list.push(path);
            }
        }
    }
    Ok(())
}

pub struct Autogrzybke {
    autogrzybke_impl: Mutex<AutogrzybkeImpl>,
}
impl Autogrzybke {
    pub fn new(resources_path: &str) -> Self {
        Autogrzybke {
            autogrzybke_impl: Mutex::new(AutogrzybkeImpl::new(resources_path)),
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
