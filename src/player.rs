use log::*;
use std::io::Write;
use std::process::{Child, Command};
use std::sync::Mutex;
use tempfile::NamedTempFile;

enum PlayerState {
    Paused {},
    Playing {
        description: String,
        worker_process: Child,
        #[allow(dead_code)]
        playlist_handle: Option<NamedTempFile>,
    },
}
struct PlaybackCommand {
    command: Command,
    description: String,
    playlist_handle: Option<NamedTempFile>,
}

impl PlaybackCommand {
    pub fn from_url(url: String) -> Self {
        let mut command = Command::new("ffplay");
        command
            .arg("-autoexit")
            .arg("-nodisp")
            .arg("-fflags")
            .arg("nobuffer")
            .arg(url.clone());
        PlaybackCommand {
            command: command,
            description: url,
            playlist_handle: None,
        }
    }

    pub fn from_files(playlist: Vec<String>) -> Result<Self, std::io::Error> {
        let mut playlist_file = tempfile::NamedTempFile::new()?;
        for file in &playlist {
            writeln!(playlist_file, "file {}", file)?;
        }
        playlist_file.flush()?;
        let mut command = Command::new("ffplay");
        command
            .arg("-autoexit")
            .arg("-nodisp")
            .arg("-f")
            .arg("concat")
            .arg("-safe")
            .arg("0")
            .arg("-i")
            .arg(playlist_file.path());

        Ok(PlaybackCommand {
            command: command,
            description: playlist_file.path().to_string_lossy().to_string(),
            playlist_handle: Some(playlist_file),
        })
    }
}

impl PlayerState {
    pub fn play(&mut self, mut playback_command: PlaybackCommand) -> Result<(), std::io::Error> {
        info!("Play {}", playback_command.description);

        match self {
            PlayerState::Playing {
                description: content_url,
                ..
            } => {
                if *content_url == playback_command.description {
                    info!("Already playing {}", *content_url);
                    Ok(())
                } else {
                    self.pause()?;
                    self.play(playback_command)
                }
            }
            PlayerState::Paused {} => {
                info!("Restart raspotify.service to make sure audio card is available");
                Command::new("sudo").arg("systemctl").arg("restart").arg("raspotify.service").spawn()?;
                info!("Start playing {}", playback_command.description);
                let spawn_result = playback_command.command.spawn()?;
                *self = PlayerState::Playing {
                    description: playback_command.description,
                    worker_process: spawn_result,
                    playlist_handle: playback_command.playlist_handle
                };
                Ok(())
            }
        }
    }

    pub fn pause(&mut self) -> Result<(), std::io::Error> {
        match self {
            PlayerState::Playing {
                worker_process,
                description,
                ..
            } => {
                info!("Pause {}", *description);
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
        self.state
            .lock()
            .unwrap()
            .play(PlaybackCommand::from_url(new_content_url))
    }

    pub fn play_local_playlist(&self, playlist: Vec<String>) -> Result<(), std::io::Error> {
        self.state
            .lock()
            .unwrap()
            .play(PlaybackCommand::from_files(playlist)?)
    }

    pub fn pause(&self) -> Result<(), std::io::Error> {
        self.state.lock().unwrap().pause()
    }
}
