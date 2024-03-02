use regex::Regex;
use simple_error::bail;
use std::process::Command;
use std::sync::Mutex;

pub struct VolumeControler {
    lock: Mutex<()>,
}

impl VolumeControler {
    pub fn new() -> VolumeControler {
        VolumeControler {
            lock: Mutex::new(()),
        }
    }
}

impl VolumeControler {
    pub fn change_volume(
        self: &VolumeControler,
        delta_percent: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _guard = self.lock.lock().unwrap();
        let vol = get_current_volume().expect("Failed to get current volume");
        set_current_volume((vol + delta_percent).clamp(0, 100)).into()
    }
}

fn get_current_volume() -> Result<i32, Box<dyn std::error::Error>> {
    let output = String::from_utf8(
        Command::new("amixer")
            .args(["sget", "SoftMaster"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let re = Regex::new(r"\[(?<percent>\d+)%\]").unwrap();
    let caps = re.captures(&*output).unwrap();
    let percent = &caps["percent"];
    let result: i32 = percent.parse()?;

    println!("Current volume: {}", result);
    return Ok(result);
}

fn set_current_volume(vol: i32) -> Result<(), Box<dyn std::error::Error>> {
    let vol_percent = vol.to_string() + "%";

    match Command::new("amixer")
        .args(["sset", "SoftMaster", &*vol_percent])
        .status()
    {
        Ok(exit_status) => {
            if exit_status.success() {
                Ok(())
            } else {
                bail!("amixer exit status: {}", exit_status)
            }
        }
        Err(err) => Err(err.into()),
    }
}
