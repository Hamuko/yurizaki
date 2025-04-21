extern crate anitomy;

use anitomy::{Anitomy, ElementCategory};

#[cfg(feature = "regex")]
use regex::Captures;

#[derive(Debug, PartialEq)]
pub enum EpisodeType {
    Ending,
    Episode,
    Movie,
    Opening,
    Other,
    OVA,
    Preview,
    Special,
}

impl EpisodeType {
    fn from_element(value: Option<&str>) -> EpisodeType {
        match value {
            None => EpisodeType::Episode,
            Some("ED") => EpisodeType::Ending,
            Some("Gekijouban") => EpisodeType::Movie,
            Some("Movie") => EpisodeType::Movie,
            Some("OP") => EpisodeType::Opening,
            Some("OVA") => EpisodeType::OVA,
            Some("Preview") => EpisodeType::Preview,
            Some("PV") => EpisodeType::Preview,
            Some("SP") => EpisodeType::Special,
            Some("TV") => EpisodeType::Episode,
            _ => EpisodeType::Other,
        }
    }
}

#[derive(Debug)]
pub struct Release {
    pub title: String,
    pub group: String,
    pub episode: String,
    pub version: i32,
    pub episode_type: EpisodeType,
}

impl Release {
    pub fn from(filename: &str) -> Option<Release> {
        let mut anitomy = Anitomy::new();
        let elements = match anitomy.parse(filename) {
            Ok(elements) => elements,
            Err(elements) => elements,
        };

        let title = elements.get(ElementCategory::AnimeTitle)?.to_string();
        let group = elements.get(ElementCategory::ReleaseGroup)?.to_string();
        let episode = elements
            .get(ElementCategory::EpisodeNumber)
            .map_or_else(|| "-1".to_string(), |v| v.to_string());
        let version: i32 = elements
            .get(ElementCategory::ReleaseVersion)
            .map_or(1, |v| v.parse().unwrap_or(1));
        let episode_type = EpisodeType::from_element(elements.get(ElementCategory::AnimeType));
        Some(Release {
            title,
            group,
            episode,
            version,
            episode_type,
        })
    }

    #[cfg(feature = "regex")]
    pub fn from_captures(title: &str, captures: Captures) -> Option<Release> {
        let group = captures.name("group")?.as_str();
        let episode = captures.name("episode")?.as_str();
        let version: i32 = match captures.name("version") {
            Some(version) => version.as_str().parse().unwrap_or(1),
            None => 1,
        };
        Some(Release {
            title: title.to_string(),
            group: group.to_string(),
            episode: episode.to_string(),
            version,
            episode_type: EpisodeType::Episode,
        })
    }

    pub fn numerical_episode(&self) -> Option<i32> {
        self.episode.parse().ok()
    }
}
