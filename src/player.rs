use std::process::{Child, Command};
use std::sync::Mutex;


pub struct Player {
    player_instance: Mutex<Option<Child>>,
}

impl Player {
    pub fn new() -> Player {
        Player {
            player_instance: Mutex::new(None),
        }
    }
}

impl Player {
    pub fn play(&self) -> Result<(), std::io::Error> {
        let player = &mut *self.player_instance.lock().unwrap();
        match player {
            Some(_) => Ok(()),
            None => {
                let spawn_result = Command::new("cvlc")
                    .arg("https://r.dcs.redcdn.pl/sc/o2/Eurozet/live/chillizet.livx")
                    .spawn()?;
                *player = Some(spawn_result);
                Ok(())
            }
        }
    }
}

impl Player {
    pub fn pause(&self) -> Result<(), std::io::Error> {
        let player = &mut *self.player_instance.lock().unwrap();
        match player {
            Some(process) => {
                process.kill()?;
                let _ = process.wait();
                *player = None;
                Ok(())
            }
            None => Ok(()),
        }
    }
}