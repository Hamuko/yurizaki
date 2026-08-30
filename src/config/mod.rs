#[cfg(feature = "directories")]
extern crate directories;

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::{env, fmt, io};

#[cfg(feature = "regex")]
use regex::Regex;

mod yaml;

type StringVec = Vec<String>;

type RuleList = Vec<Rule>;
type RuleMapping = HashMap<String, usize>;

#[cfg(feature = "regex")]
type RuleRegexes = Vec<(Regex, usize)>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    MissingLibrary,
    MissingSource,
    Yaml(serde_yaml::Error),
}

#[derive(Debug)]
pub struct Configuration {
    pub rules: RuleList,
    mapping: RuleMapping,
    pub source: PathBuf,
    pub library: PathBuf,
    pub debounce: u64,

    #[cfg(feature = "trash")]
    pub trash: bool,

    #[cfg(feature = "regex")]
    pub regexes: RuleRegexes,
}

fn get_config_from_args() -> Option<PathBuf> {
    let path = env::args().nth(1)?;
    Some(PathBuf::from(path))
}

#[cfg(feature = "directories")]
pub fn get_path() -> Option<PathBuf> {
    if let Some(config_path) = get_config_from_args() {
        return Some(config_path);
    }
    let project_directory = directories::ProjectDirs::from("", "", "yurizaki")?;
    let mut config_path = PathBuf::new();
    config_path.push(project_directory.config_dir());
    config_path.push("config.yml");
    Some(config_path)
}

#[cfg(not(feature = "directories"))]
pub fn get_path() -> Option<PathBuf> {
    return get_config_from_args();
}

impl Configuration {
    pub fn new(path: &PathBuf) -> Result<Configuration, Error> {
        let file = File::open(&path).map_err(Error::Io)?;
        let parsed: yaml::Config = serde_yaml::from_reader(file).map_err(Error::Yaml)?;

        let source = match parsed.source {
            Some(value) => value,
            None => return Err(Error::MissingSource),
        };
        let library = match parsed.library {
            Some(value) => value,
            None => return Err(Error::MissingLibrary),
        };
        let debounce = parsed.debounce.unwrap_or(60);

        let library = PathBuf::from(library);
        let mut mapping: RuleMapping = RuleMapping::new();
        let mut rules = RuleList::new();

        #[cfg(feature = "regex")]
        let mut regexes = RuleRegexes::new();

        for (title, rule) in parsed.rules {
            let rule_index = rules.len();
            mapping.insert(title.clone(), rule_index);
            for alias in &rule.aliases {
                mapping.insert(alias.clone(), rule_index);
            }

            #[cfg(feature = "regex")]
            for regex_string in &rule.regex {
                if let Some(regex) = Self::parse_regex(regex_string) {
                    regexes.push((regex, rule_index));
                }
            }

            let groups = rule.groups;
            let minimum_episode = rule.minimum.episode_number;
            rules.push(Rule {
                groups,
                title,
                minimum_episode,
            });
        }

        if cfg!(not(feature = "trash")) && parsed.trash {
            log::warn!("yurizaki was built without trash support; enabling trash does nothing.");
        }

        let source = PathBuf::from(source);

        Ok(Configuration {
            debounce,
            library,
            mapping,
            rules,
            source,
            #[cfg(feature = "trash")]
            trash: parsed.trash,
            #[cfg(feature = "regex")]
            regexes,
        })
    }

    pub fn get_rule(&self, name: &str) -> Option<&Rule> {
        Some(&self.rules[*self.mapping.get(name)?])
    }

    #[cfg(feature = "regex")]
    fn parse_regex(regex_string: &str) -> Option<Regex> {
        let Ok(regex) = Regex::new(regex_string) else {
            log::warn!("String '{}' could not be parsed as regex", regex_string);
            return None;
        };
        let captures = regex.capture_names().flatten().collect::<Vec<&str>>();
        let has_episode = captures.iter().any(|e| e == &"episode");
        let has_group = captures.iter().any(|e| e == &"group");
        if has_episode && has_group {
            Some(regex)
        } else {
            if !has_episode {
                log::warn!(
                    "Could not use regex '{}': no capture group for 'episode'",
                    regex,
                );
            }
            if !has_group {
                log::warn!(
                    "Could not use regex '{}': no capture group for 'group'",
                    regex,
                );
            }
            None
        }
    }
}

impl fmt::Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rules = self
            .rules
            .iter()
            .map(|rule| format!("- {}", rule))
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{}", rules)
    }
}

#[derive(Debug)]
pub struct Rule {
    pub groups: StringVec,
    pub title: String,
    pub minimum_episode: Option<i64>,
}

impl Rule {
    pub fn get_priority(&self, group_name: &str) -> Option<usize> {
        self.groups.iter().position(|x| x == group_name)
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let groups = self.groups.join(", ");
        write!(f, "{} ({})", self.title, groups)
    }
}
