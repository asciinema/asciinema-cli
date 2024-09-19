use super::Command;
use crate::asciicast;
use crate::cli;
use crate::config::Config;
use crate::logger;
use crate::player::{self, KeyBindings};
use crate::tty;
use crate::util;
use anyhow::Result;

impl Command for cli::Play {
    fn run(self, config: &Config) -> Result<()> {
        let speed = self.speed.or(config.cmd_play_speed()).unwrap_or(1.0);
        let idle_time_limit = self.idle_time_limit.or(config.cmd_play_idle_time_limit());

        logger::info!("Replaying session from {}", self.filename);

        let path = util::get_local_path(&self.filename)?;

        let ended = loop {
            let recording = asciicast::open_from_path(&*path)?;
            let tty = tty::DevTty::open()?;
            let keys = get_key_bindings(config)?;

            let ended = player::play(
                recording,
                tty,
                speed,
                idle_time_limit,
                self.pause_on_markers,
                &keys,
                self.resize,
            )?;

            if !self.loop_ {
                break ended;
            }
        };

        if ended {
            logger::info!("Playback ended");
        } else {
            logger::info!("Playback interrupted");
        }

        Ok(())
    }
}

fn get_key_bindings(config: &Config) -> Result<KeyBindings> {
    let mut keys = KeyBindings::default();

    if let Some(key) = config.cmd_play_pause_key()? {
        keys.pause = key;
    }

    if let Some(key) = config.cmd_play_step_key()? {
        keys.step = key;
    }

    if let Some(key) = config.cmd_play_next_marker_key()? {
        keys.next_marker = key;
    }

    Ok(keys)
}
