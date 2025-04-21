#[cfg(feature = "trash")]
extern crate trash;

use log::{debug, error, info, warn, LevelFilter};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use simple_logger::SimpleLogger;

mod anime;
mod config;

#[derive(Debug)]
enum ActionCategory {
    ReloadConfig,
    Process,
}

#[derive(Debug)]
struct Action {
    category: ActionCategory,
    path: PathBuf,
}

enum ExistingRelease {
    Inferior(PathBuf),
    Superior(PathBuf),
}

fn entry_to_file_path(entry: io::Result<fs::DirEntry>) -> Option<PathBuf> {
    let entry = entry.ok()?;
    let metadata = entry.metadata().ok()?;
    if metadata.is_dir() {
        return None;
    }
    Some(entry.path())
}

fn scan_directory(config: &config::Configuration) {
    for entry in fs::read_dir(&config.source).unwrap() {
        if let Some(path) = entry_to_file_path(entry) {
            handle_file(&config, path);
        }
    }
}

fn find_existing_release(
    path: &PathBuf,
    release: anime::Release,
    rule: &config::Rule,
    config: &config::Configuration,
) -> Option<ExistingRelease> {
    let group_priority = rule.get_priority(&release.group).unwrap();
    let episode_number = release.numerical_episode();

    for entry in fs::read_dir(path).unwrap() {
        let Some(path) = entry_to_file_path(entry) else {
            continue;
        };
        let Some(filename) = path.file_name() else {
            continue;
        };
        let Some(filename) = filename.to_str() else {
            continue;
        };
        let Some(entry_release) = make_release(config, filename) else {
            continue;
        };

        // Check that the entry is of the same type as the given release, since we
        // wouldn't want to match "Anime - OVA1" to "Anime - 01".
        if entry_release.episode_type != release.episode_type {
            continue;
        }

        // Check that entry is the same episode as the given release.
        match (episode_number, entry_release.numerical_episode()) {
            (Some(episode_number), Some(entry_episode_number)) => {
                if entry_episode_number != episode_number {
                    continue;
                }
            }
            (Some(_), None) | (None, Some(_)) => continue,
            (None, None) => {
                if entry_release.episode != release.episode {
                    continue;
                }
            }
        }

        // Check if library contains a release from a group with a higher priority.
        let entry_group_priority = match rule.get_priority(&entry_release.group) {
            Some(position) => position,
            None => continue,
        };
        if entry_group_priority < group_priority {
            // Entry's release group is listed higher (has lower index).
            return Some(ExistingRelease::Superior(path));
        } else if entry_group_priority > group_priority {
            // Entry's release group is listed lower (has higher index).
            return Some(ExistingRelease::Inferior(path));
        } else if release.version < entry_release.version {
            // Entry is a greater version.
            return Some(ExistingRelease::Superior(path));
        } else if release.version > entry_release.version {
            // Entry is a lesser version.
            return Some(ExistingRelease::Inferior(path));
        }
        // If the control flow reaches here, we have the same episode for the same group
        // priority and for the same release version. Either we are dealing with the same
        // releases or there's a need for additional checks.
    }
    None
}

fn get_filesize(path: &PathBuf) -> Option<i64> {
    let metadata = fs::metadata(path).ok()?;
    Some(metadata.len() as i64)
}

fn should_recopy(from: &PathBuf, to: &PathBuf) -> bool {
    let Some(to_filesize) = get_filesize(to) else {
        // Failed to get filesize of destination file, so recopying just in case.
        return true;
    };
    let from_filesize = get_filesize(from);
    if from_filesize != Some(to_filesize) {
        return true;
    }
    false
}

#[cfg(feature = "trash")]
fn remove_file(config: &config::Configuration, path: &PathBuf) {
    if config.trash {
        match trash::delete(path) {
            Ok(_) => {
                debug!("Removed file \"{}\"", path.display());
            }
            Err(e) => {
                warn!("Unable to trash \"{}\" ({:?})", path.display(), e);
            }
        }
    } else {
        match fs::remove_file(path) {
            Ok(_) => {
                debug!("Removed file \"{}\"", path.display());
            }
            Err(e) => {
                warn!("Unable to delete \"{}\" ({})", path.display(), e);
            }
        }
    }
}

#[cfg(not(feature = "trash"))]
fn remove_file(_config: &config::Configuration, path: &PathBuf) {
    match fs::remove_file(path) {
        Ok(_) => {
            debug!("Removed file \"{}\"", path.display());
        }
        Err(e) => {
            warn!("Unable to delete \"{}\" ({})", path.display(), e);
        }
    }
}

fn make_release(config: &config::Configuration, filename: &str) -> Option<anime::Release> {
    #[cfg(feature = "regex")]
    for (regex, index) in &config.regexes {
        if let Some(captures) = regex.captures(filename) {
            debug!("Matched {} to regex {}", filename, regex);
            let rule = &config.rules[*index];
            if let Some(release) = anime::Release::from_captures(&rule.title, captures) {
                return Some(release);
            }
        }
    }

    anime::Release::from(filename)
}

fn handle_file(config: &config::Configuration, path: PathBuf) -> Option<()> {
    if !path.exists() {
        // File doesn't actually exist, so let's bail out.
        return None;
    }

    let filename = &path.file_name()?.to_str()?;
    let release = make_release(config, filename)?;
    let rule = config.get_rule(&release.title)?;
    info!("MATCH: \"{}\" => {}", &filename, rule.title);

    if !rule.groups.contains(&release.group) {
        info!(
            "SKIP: Group \"{}\" not listed in {}",
            release.group, rule.title
        );
        return None;
    }

    // Check minimum episode threshold.
    match (rule.minimum.episode_number, release.numerical_episode()) {
        (Some(minimum), Some(episode_number)) => {
            if minimum > episode_number as i64 {
                info!(
                    "SKIP: Episode number {} does not meet minimum of {}",
                    episode_number, minimum
                );
                return None;
            }
        }
        _ => {}
    }

    if get_filesize(&path) == Some(0) {
        // Avoid working on zero-length files (if a file was
        // for example pre-allocated before writing).
        return None;
    }

    let mut copy_target = config.library.clone();
    copy_target.push(&rule.title);
    let target_directory = copy_target.clone();
    if !target_directory.exists() {
        debug!(
            "Missing directory \"{}\", creating...",
            &target_directory.to_str().unwrap()
        );
        match fs::create_dir(&target_directory) {
            Ok(()) => {
                debug!(
                    "Directory \"{}\" created",
                    &target_directory.to_str().unwrap()
                );
            }
            Err(error) => {
                error!(
                    "Unable to create directory \"{}\" ({}), skipping file...",
                    &target_directory.display(),
                    error
                );
                return None;
            }
        };
    }
    copy_target.push(filename);

    let mut copy_file = true;
    if copy_target.exists() {
        if should_recopy(&path, &copy_target) {
            info!(
                "COPY: {} exists in destination, but fails comparison",
                filename
            );
        } else {
            info!(
                "SKIP: {} exists in destination and passes comparison",
                filename
            );
            copy_file = false;
        }
    } else {
        match find_existing_release(&target_directory, release, rule, config) {
            Some(ExistingRelease::Superior(path)) => {
                info!("Superior release found: \"{}\"", path.display());
                copy_file = false;
            }
            Some(ExistingRelease::Inferior(path)) => {
                info!("Inferior release found: \"{}\"", path.display());
                remove_file(&config, &path);
            }
            None => info!("No other release"),
        }
    }
    if copy_file {
        match fs::copy(&path, &copy_target) {
            Ok(_) => {
                info!("Copied \"{}\" to \"{}\"", filename, &copy_target.display());
            }
            Err(_) => {
                error!("Failed to copy \"{}\"", filename);
            }
        };
    }
    Some(())
}

fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .env()
        .init()
        .unwrap();

    let Some(config_path) = config::get_path() else {
        error!("Could not establish configuration directory.");
        process::exit(1);
    };
    debug!("Loading configuration from \"{}\"", config_path.display());
    let mut configuration = match config::Configuration::new(&config_path) {
        Ok(config) => config,
        Err(config::Error::Io(error)) => {
            match error.kind() {
                io::ErrorKind::NotFound => {
                    error!(
                        "Could not find the configuration file in \"{}\".",
                        config_path.display()
                    );
                }
                _ => {
                    error!(
                        "There was a problem with reading the configuration: {}",
                        error
                    );
                }
            }
            return;
        }
        Err(config::Error::MissingSource) => {
            error!("Configuration file is missing a source path");
            process::exit(1);
        }
        Err(config::Error::MissingLibrary) => {
            error!("Configuration file is missing a library path");
            process::exit(1);
        }
        Err(config::Error::YamlError) => {
            error!("There was a problem with reading the configuration Yaml file");
            process::exit(1);
        }
    };

    let (watch_tx, watch_rx) = channel();
    let mut config_watcher: RecommendedWatcher =
        watcher(watch_tx.clone(), Duration::from_secs(5)).unwrap();
    config_watcher
        .watch(&config_path, RecursiveMode::NonRecursive)
        .unwrap();

    // TODO: Configurable debounce time (if it's even needed).
    let debounce_duration = Duration::from_secs(60);
    let mut watcher: RecommendedWatcher = watcher(watch_tx, debounce_duration).unwrap();
    match watcher.watch(&configuration.source, RecursiveMode::NonRecursive) {
        Ok(()) => {}
        Err(notify::Error::PathNotFound) => {
            error!(
                "Could not watch source path \"{}\". \
                Please verify that the `source` configuration value is set correctly.",
                configuration.source.display()
            );
            process::exit(1);
        }
        Err(error) => {
            error!("Source watch error: {}", error);
            process::exit(1);
        }
    };

    // Perform initial scan after.
    scan_directory(&configuration);

    let (action_tx, action_rx) = channel();
    let cloned_config_path = config_path.clone();
    thread::spawn(move || loop {
        let event = match watch_rx.recv() {
            Ok(event) => event,
            Err(e) => {
                error!("Watch error: {}", e);
                continue;
            }
        };
        let path = match event {
            DebouncedEvent::Create(path) => path,
            DebouncedEvent::Write(path) => path,
            _ => continue,
        };

        let action = if path == cloned_config_path {
            Action {
                category: ActionCategory::ReloadConfig,
                path: path.clone(),
            }
        } else {
            Action {
                category: ActionCategory::Process,
                path: path.clone(),
            }
        };

        let send_result = action_tx.send(action);
        if send_result.is_err() {
            error!("Unable to send action notification for {}", path.display());
        }
    });

    loop {
        let action = match action_rx.recv() {
            Ok(action) => action,
            Err(_) => continue,
        };
        match action.category {
            ActionCategory::ReloadConfig => {
                let new_configuration = match config::Configuration::new(&config_path) {
                    Ok(config) => config,
                    Err(_) => {
                        warn!("Unable to reload configuration. Old configuration will be used instead.");
                        continue;
                    }
                };
                configuration = new_configuration;
                info!("Reloaded configuration:\n{}", configuration);
                scan_directory(&configuration);
            }
            ActionCategory::Process => {
                handle_file(&configuration, action.path);
            }
        }
    }
}
