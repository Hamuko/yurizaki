extern crate yaml_rust;

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;

use yaml_rust::{yaml, Yaml, YamlLoader};

type StringVec = Vec<String>;

trait StringVecMethods {
    fn yaml_array_to_vec(array: &Vec<Yaml>) -> Option<StringVec>;
}

impl StringVecMethods for StringVec {
    fn yaml_array_to_vec(array: &Vec<Yaml>) -> Option<StringVec> {
        let mut vec = StringVec::new();
        for item in array {
            let Some(item) = item.as_str() else { continue };
            vec.push(item.to_string());
        }
        Some(vec)
    }
}

type RuleList = Vec<Rule>;
type RuleMapping = HashMap<String, usize>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    MissingLibrary,
    MissingSource,
    YamlError,
}

#[derive(Debug)]
pub struct Configuration {
    rules: RuleList,
    mapping: RuleMapping,
    pub source: PathBuf,
    pub library: PathBuf,
    pub trash: bool,
}

impl Configuration {
    pub fn new(path: &PathBuf) -> Result<Configuration, Error> {
        let file_content = match load_file_to_string(path) {
            Ok(string) => string,
            Err(e) => return Err(Error::Io(e)),
        };
        let yaml_vector = match YamlLoader::load_from_str(&file_content) {
            Ok(config) => config,
            Err(_) => return Err(Error::YamlError),
        };
        let yaml_document = &yaml_vector.get(0);
        let configuration_yaml = match yaml_document {
            Some(yaml) => yaml,
            None => return Err(Error::YamlError),
        };

        let mut library: Option<PathBuf> = None;
        let mut mapping: RuleMapping = RuleMapping::new();
        let mut rules = RuleList::new();
        let mut source_path: Option<String> = None;
        let mut trash: bool = false;

        if let Some(configuration_yaml) = configuration_yaml.as_hash() {
            for (key, value) in configuration_yaml {
                match (key.as_str(), value) {
                    (Some("library"), Yaml::String(value)) => {
                        library = Some(PathBuf::from(value));
                    }
                    (Some("source"), Yaml::String(value)) => {
                        source_path = Some(value.clone());
                    }
                    (Some("trash"), Yaml::Boolean(value)) => {
                        trash = *value;
                    }
                    (Some(title), Yaml::Hash(hash)) => {
                        let title = title.to_string();
                        let Some(rule) = Rule::read(hash, title.clone()) else { continue };
                        rules.push(rule);
                        let rule_index = rules.len() - 1;
                        mapping.insert(title, rule_index);

                        let blank_vec = Vec::new();
                        for alias in value["aliases"].as_vec().unwrap_or(&blank_vec) {
                            let Some(alias) = alias.as_str() else { continue };
                            mapping.insert(alias.to_string(), rule_index);
                        }
                    }
                    _ => {}
                }
            }
        }

        let source_path = match source_path {
            Some(value) => value,
            None => return Err(Error::MissingSource),
        };
        let library = match library {
            Some(value) => value,
            None => return Err(Error::MissingLibrary),
        };

        if cfg!(not(feature = "trash")) && trash {
            println!(
                "yurizaki was built without trash support; enabling trash does nothing."
            );
        }

        Ok(Configuration {
            library: library,
            mapping: mapping,
            rules: rules,
            source: PathBuf::from(source_path),
            trash: trash,
        })
    }

    pub fn get_rule(&self, name: &str) -> Option<&Rule> {
        Some(&self.rules[*self.mapping.get(name)?])
    }
}

#[derive(Debug)]
pub struct Rule {
    pub groups: StringVec,
    pub title: String,
    pub minimum: Option<i64>,
}

impl Rule {
    fn read(config: &yaml::Hash, title: String) -> Option<Self> {
        let mut groups: StringVec = Vec::new();
        let mut minimum: Option<i64> = None;
        for (key, value) in config {
            match (key.as_str(), value) {
                (Some("groups"), Yaml::Array(array)) => {
                    if let Some(vec) = StringVec::yaml_array_to_vec(&array) {
                        groups = vec;
                    }
                }
                (Some("minimum"), Yaml::Integer(integer)) => {
                    minimum = Some(*integer);
                }
                _ => (),
            }
        }
        Some(Rule {
            title: title,
            groups: groups,
            minimum: minimum,
        })
    }

    pub fn get_priority(&self, group_name: &str) -> Option<usize> {
        self.groups.iter().position(|x| x == group_name)
    }
}

fn load_file_to_string(path: &PathBuf) -> Result<String, io::Error> {
    let mut file = File::open(&path)?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;
    Ok(file_content)
}
