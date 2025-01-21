use anyhow::{anyhow, Context};
use log::*;
use regex::Regex;
use std::process::Command;
use std::sync::Mutex;

pub struct VolumeController {
    lock: Mutex<()>,
}

impl VolumeController {
    pub fn new() -> VolumeController {
        VolumeController {
            lock: Mutex::new(()),
        }
    }
}

impl VolumeController {
    pub fn change_volume(self: &VolumeController, delta_percent: i32) -> Result<(), anyhow::Error> {
        let _guard = self
            .lock
            .lock()
            .or_else(|e| Err(anyhow!("VolumeController mutex poisoned: {e:#?}")))?;
        let vol = get_current_volume().context("Failed to get current volume")?;
        set_current_volume((vol + delta_percent).clamp(0, 100)).into()
    }
}

fn get_current_volume() -> Result<i32, anyhow::Error> {
    let output = String::from_utf8(
        Command::new("amixer")
            .args(["sget", "SoftMaster"])
            .output()?
            .stdout,
    )
    .context("Get volume with amixer failed")?;
    let re = Regex::new(r"\[(?<percent>\d+)%]")?;
    let caps = re
        .captures(&*output)
        .ok_or(anyhow!("Unable to parse current volume from: {:?}", output))?;
    let percent = &caps["percent"];
    let result: i32 = percent.parse()?;

    info!("Current volume: {}", result);
    Ok(result)
}

fn set_current_volume(vol: i32) -> Result<(), anyhow::Error> {
    let vol_percent = vol.to_string() + "%";

    match Command::new("amixer")
        .args(["sset", "SoftMaster", &*vol_percent])
        .status()
    {
        Ok(exit_status) => {
            if exit_status.success() {
                Ok(())
            } else {
                Err(anyhow!("amixer exit status: {}", exit_status))
                    .context("Set current volume failed")
            }
        }
        Err(err) => Err(anyhow!(err).context("Set volume with amixer failed")),
    }
}
