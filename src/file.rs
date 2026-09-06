use notify_debouncer_mini::{
    notify::*, DebounceEventHandler, DebounceEventResult, DebouncedEventKind,
};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub enum ActionCategory {
    ReloadConfig,
    Process,
}

#[derive(Debug)]
pub struct Action {
    pub category: ActionCategory,
    pub path: PathBuf,
}

pub struct EventHandler {
    pub config_path: PathBuf,
    pub tx: Sender<Action>,
    pub debounce: Duration,
}

impl DebounceEventHandler for EventHandler {
    fn handle_event(&mut self, result: DebounceEventResult) {
        match result {
            Ok(debounced_events) => {
                for debounced_event in debounced_events {
                    if debounced_event.kind == DebouncedEventKind::AnyContinuous {
                        continue; // Skip continuous events in case the download/transfer is still happening.
                    }
                    if debounced_event.path.try_exists().is_ok_and(|x| !x) {
                        log::debug!(
                            "Skipping {}: file does not exist",
                            debounced_event.path.display()
                        );
                        continue;
                    }
                    match get_file_modified_duration(&debounced_event.path) {
                        Ok(modified_since) => {
                            if modified_since > self.debounce {
                                log::debug!(
                                    "Skipping {}: modified too long ago",
                                    debounced_event.path.display()
                                );
                                continue;
                            }
                        }
                        Err(error) => {
                            log::warn!(
                                "Could not get modified time for {}: {}",
                                debounced_event.path.display(),
                                error
                            );
                        }
                    };
                    let action = if debounced_event.path == self.config_path {
                        Action {
                            category: ActionCategory::ReloadConfig,
                            path: self.config_path.clone(),
                        }
                    } else {
                        Action {
                            category: ActionCategory::Process,
                            path: debounced_event.path.clone(),
                        }
                    };
                    let send_result = self.tx.send(action);
                    if send_result.is_err() {
                        log::error!(
                            "Unable to send action notification for {}",
                            debounced_event.path.display()
                        );
                    }
                }
            }
            Err(e) => println!("Error {:?}", e),
        }
    }
}

fn get_file_modified_duration(path: &PathBuf) -> Result<Duration> {
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    let now = SystemTime::now();
    let duration = match now.duration_since(modified) {
        Ok(duration) => duration,
        Err(error) => {
            log::warn!("Could not calculate modified time: {}", error);
            return Err(Error::generic(
                "Unexpected duration calculation error. Maybe your computer is fucked?",
            ));
        }
    };
    Ok(duration)
}

pub fn get_filesize(path: &PathBuf) -> Option<i64> {
    let metadata = fs::metadata(path).ok()?;
    Some(metadata.len() as i64)
}
