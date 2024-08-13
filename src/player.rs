use std::process::{Child, Command};
use std::sync::Mutex;

enum PlayerState {
    Paused {},
    Playing {
        content_url: String,
        worker_process: Child,
    },
}

pub struct Player {
    state: Mutex<PlayerState>,
}

impl Player {
    pub fn new() -> Player {
        Player {
            state: Mutex::new(PlayerState::Paused {}),
        }
    }
}

impl Player {
    pub fn play(&self, new_content_url: String) -> Result<(), std::io::Error> {
        let state = &mut *self.state.lock().unwrap();
        match state {
            PlayerState::Playing { content_url, .. } => {
                if *content_url == new_content_url {
                    Ok(())
                } else {
                    self.pause()?;
                    self.play(new_content_url)
                }
            }
            PlayerState::Paused {} => {
                let spawn_result = Command::new("mpv")
                    .arg(new_content_url.clone())
                    .spawn()?;
                *state = PlayerState::Playing { content_url: new_content_url, worker_process: spawn_result };
                Ok(())
            }
        }
    }
}

impl Player {
    pub fn pause(&self) -> Result<(), std::io::Error> {
        let state = &mut *self.state.lock().unwrap();
        match state {
            PlayerState::Playing{ worker_process, ..} => {
                worker_process.kill()?;
                let _ = worker_process.wait();
                *state = PlayerState::Paused {};
                Ok(())
            }
            PlayerState::Paused {} => Ok(()),
        }
    }
}