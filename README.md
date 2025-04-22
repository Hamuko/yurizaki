# yurizaki

yurizaki is a simple stateless file organizer daemon for anime releases.

## About

yurizaki is designed to be ran in the background as a daemon. Any changes to the configuration file will cause the program to hot-reload the configuration as long as the configuration is valid.

The daemon operates statelessly, meaning it does not store information about what files it has seen or operated on anywhere. Instead the program will scan all files on startup and configuration reload, apply any possible rules and continue to monitor the source directory for new files.

The main goal of yurizaki is to ease the management of releases from different groups. Each anime can have a ranked listing of possible groups and yurizaki will replace releases from lower-ranked groups with releases from higher-ranked groups. It will also check release versions and replace an older version from the same release group. The end result is that your library should ever only have one version of a singular episode.

Release information (anime title, episode number, release group, release version) is parsed with [anitomy](https://github.com/erengy/anitomy) for maximum compatibility with different naming schemes.

## Configuration

yurizaki is configured with [YAML](https://en.wikipedia.org/wiki/YAML). This configuration file should be added to `~/.config/yurizaki/config.yml` on Linux or `~/Library/Application Support/yurizaki/config.yml` on macOS.

Top-level of the configuration requires two values: `library` and `source`. `library` is the path to the directory to where releases should be copied and `source` is the source directory from where files are copied.

You can also set an optional `trash` boolean value on whether or not old files are moved to the trash or fully deleted.

### Rules

All other settings in the configuration file should be matching rules, dictionaries where the key is the main title of the anime, and will be used as the target directory inside the library path (`/library/Main title`).

Names of the release groups should be listed under the `groups` key for every rule. Groups are an ordered list of all possible groups that can be matched from best to worst.

Other possible anime titles to match against can be listed under the optional `aliases` key. If different release groups use different titles in their filenames, aliases will be used to supplement the matching logic.

For really tricky cases where anitomy parsing fails, you can compile a list of regular expressions under the `regex` key in each rule. The regular expressions must contain capture groups for `episode` and `group` in order to match episode numbers and release groups. They may also optionally include a capture group `version`. Regular parsing will still be used as a fallback, so you can have automatic parsing and regex parsing for different groups in a single rule.

It's also possible to exclude prior episodes from the matching logic by giving an episode number for the `episode` key under the `minimum` key. All episodes must have an episode number equal or greater than this value to be copied. This is useful for separating split cours.

### Example configuration

```yaml
source: /src
library: /library
trash: false

# Matching rules:

Hot Pockets:
  groups:
    - Redundant-subs
    - BadSubtitles
  regex:
    - ^\[(?<group>Redundant-subs)\] Hot Pockets - (?<episode>\d+) \(S01E\d+\)

Mev-Dev Different:
  groups:
    - BadSubtitles

To Aru Himitsu no Bangumi Y:
  aliases:
    - Toaru Himitsu no Bangumi Y
    - A Certain Secret Show Y
  groups:
    - BWN
    - Edited-BadSubtitles
    - BadSubtitles

UMA Girls - Cinderella Dust:
  groups:
    - ToolSub
  regex:
    - ^UMAgirls\.Cinderella\.Dust\.S01E(?<episode>\d+)\..*-(?<group>ToolSub).mkv$

Wizard from Neptune Part 2:
  groups:
    - BWM
    - BadSubtitles
  minimum:
    episode: 13
```

## Usage

Once you have a configuration file set up, just run the binary. yurizaki will load the configuration and run the loaded rules against the source directory. After this, it'll continue to watch for changes in the configuration file and in the source directory. If you change the configuration, it will reload the configuration and run the loaded rules against the source directory. If a file is added, it will be processed according to the rules.

You can also run yurizaki in Docker using the provided images. All you need to do is run the Docker container with bind mounts for the configuration file (`/config.yml`), source directory and library directory. For example:

```shell
docker run \
    -v /path/to/config.yml:/config.yml \
    -v /path/to/src/directory:/src \
    -v /path/to/library/directory:/library \
    ghcr.io/hamuko/yurizaki:latest
```

Docker Compose:

```yaml
version: '3.7'
services:
  yurizaki:
    image: ghcr.io/hamuko/yurizaki:latest
    container_name: yurizaki
    user: "<user ID>:<group ID>"
    volumes:
      - /path/to/config.yml:/config.yml
      - /path/to/src/directory:/src
      - /path/to/library/directory:/library
    restart: on-failure
```

Remember to use the paths used inside the container (`/src` and `/library` in the above example) and not the ones used on the host machine when writing the configuration file.
