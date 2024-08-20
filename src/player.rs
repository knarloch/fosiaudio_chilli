use log::*;
use std::process::{Child, Command};
use std::sync::Mutex;

enum PlayerState {
    Paused {},
    Playing {
        content_url: String,
        worker_process: Child,
    },
}

impl PlayerState {
    pub fn play(&mut self, new_content_url: String) -> Result<(), std::io::Error> {
        info!("Play {}", new_content_url);

        match self {
            PlayerState::Playing { content_url, .. } => {
                if *content_url == new_content_url {
                    info!("Already playing {}", new_content_url);
                    Ok(())
                } else {
                    self.pause()?;
                    self.play(new_content_url)
                }
            }
            PlayerState::Paused {} => {
                info!("Start playing {}", new_content_url);
                let spawn_result = Command::new("mpv").arg(new_content_url.clone()).spawn()?;
                *self = PlayerState::Playing {
                    content_url: new_content_url,
                    worker_process: spawn_result,
                };
                Ok(())
            }
        }
    }

    pub fn pause(&mut self) -> Result<(), std::io::Error> {
        match self {
            PlayerState::Playing {
                worker_process,
                content_url,
            } => {
                info!("Pause {}", *content_url);
                worker_process.kill()?;
                let _ = worker_process.wait();
                *self = PlayerState::Paused {};
                Ok(())
            }
            PlayerState::Paused {} => {
                info!("Already Paused");
                Ok(())
            }
        }
    }
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
        self.state.lock().unwrap().play(new_content_url)
    }

    pub fn pause(&self) -> Result<(), std::io::Error> {
        self.state.lock().unwrap().pause()
    }
}
