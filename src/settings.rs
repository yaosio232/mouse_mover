use std::{fs, io, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub idle_minutes: u32,
    pub repeat_seconds: u32,
    pub transparent: bool,
    pub mode: Mode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Write,
    Random,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            idle_minutes: 3,
            repeat_seconds: 60,
            transparent: false,
            mode: Mode::Write,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        settings_path()
            .ok()
            .and_then(|path| fs::read_to_string(path).ok())
            .map_or_else(Self::default, |text| parse(&text))
    }

    pub fn save(&self) -> io::Result<()> {
        let path = settings_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serialize(*self))
    }
}

fn settings_path() -> io::Result<PathBuf> {
    let root = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("AppData").join("Local"))
        })
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local").join("share")))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "no local application data directory",
            )
        })?;
    Ok(root.join("AZuSanpo").join("settings.conf"))
}

fn parse(text: &str) -> Settings {
    let mut settings = Settings::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "idle_minutes" => {
                if let Ok(v) = value.trim().parse::<u32>() {
                    if [1, 3, 5, 10].contains(&v) {
                        settings.idle_minutes = v;
                    }
                }
            }
            "repeat_seconds" => {
                if let Ok(v) = value.trim().parse::<u32>() {
                    if [30, 60, 120, 300].contains(&v) {
                        settings.repeat_seconds = v;
                    }
                }
            }
            "transparent" => match value.trim() {
                "true" => settings.transparent = true,
                "false" => settings.transparent = false,
                _ => {}
            },
            "mode" => match value.trim() {
                "write" => settings.mode = Mode::Write,
                "random" => settings.mode = Mode::Random,
                _ => {}
            },
            _ => {}
        }
    }
    settings
}

fn serialize(settings: Settings) -> String {
    format!(
        "idle_minutes={}\nrepeat_seconds={}\ntransparent={}\nmode={}\n",
        settings.idle_minutes,
        settings.repeat_seconds,
        settings.transparent,
        match settings.mode {
            Mode::Write => "write",
            Mode::Random => "random",
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_and_out_of_menu_values_default_independently() {
        let settings = parse("idle_minutes=2\nrepeat_seconds=61\ntransparent=yes\nmode=other");
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn valid_values_round_trip_through_text() {
        let settings = Settings {
            idle_minutes: 10,
            repeat_seconds: 300,
            transparent: true,
            mode: Mode::Random,
        };
        assert_eq!(parse(&serialize(settings)), settings);
    }
}
