/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub navidrome_url: String,
    pub navidrome_user: String,
    pub navidrome_password: String,
    pub alexa_skill_id: Option<String>,
    pub port: u16,
    pub skip_signature_verification: bool,
}

/// Errors produced while loading the runtime configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing env var: {0}")]
    Missing(&'static str),
    #[error("invalid PORT: {0}")]
    InvalidPort(String),
    #[error("invalid boolean value: {0}")]
    InvalidBool(String),
}

impl Config {
    /// Loads the configuration from the environment, applying defaults for
    /// optional variables (`PORT` = 3000, no Alexa skill ID, signature checks on).
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] for missing required variables or malformed
    /// optional values.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_env_source(|key| std::env::var(key).ok())
    }

    /// Loads the configuration from an injected source of environment values.
    ///
    /// This keeps configuration tests deterministic without mutating process
    /// environment variables.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] for missing required variables or malformed
    /// optional values.
    pub fn from_env_source(getter: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let navidrome_url = getter("NAVIDROME_URL").ok_or(ConfigError::Missing("NAVIDROME_URL"))?;
        let navidrome_user =
            getter("NAVIDROME_USER").ok_or(ConfigError::Missing("NAVIDROME_USER"))?;
        let navidrome_password =
            getter("NAVIDROME_PASSWORD").ok_or(ConfigError::Missing("NAVIDROME_PASSWORD"))?;

        let port = match getter("PORT") {
            None => 3000,
            Some(raw) => raw
                .parse::<u16>()
                .map_err(|_| ConfigError::InvalidPort(raw))?,
        };
        let skip_signature_verification = match getter("SKIP_SIGNATURE_VERIFICATION") {
            None => false,
            Some(raw) => parse_bool(&raw)?,
        };

        Ok(Self {
            navidrome_url,
            navidrome_user,
            navidrome_password,
            alexa_skill_id: getter("ALEXA_SKILL_ID"),
            port,
            skip_signature_verification,
        })
    }
}

/// Parses the boolean spellings accepted for optional environment flags.
fn parse_bool(value: &str) -> Result<bool, ConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        other => Err(ConfigError::InvalidBool(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn source(entries: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    fn required() -> [(&'static str, &'static str); 3] {
        [
            ("NAVIDROME_URL", "https://n.example"),
            ("NAVIDROME_USER", "u"),
            ("NAVIDROME_PASSWORD", "p"),
        ]
    }

    #[test]
    fn defaults_applied() {
        let config = Config::from_env_source(source(&required())).unwrap();

        assert_eq!(config.port, 3000);
        assert_eq!(config.alexa_skill_id, None);
        assert!(!config.skip_signature_verification);
    }

    #[test]
    fn optional_values_are_loaded() {
        let config = Config::from_env_source(source(&[
            ("NAVIDROME_URL", "https://n.example"),
            ("NAVIDROME_USER", "u"),
            ("NAVIDROME_PASSWORD", "p"),
            ("ALEXA_SKILL_ID", "amzn1.ask.skill.1"),
            ("PORT", "8080"),
            ("SKIP_SIGNATURE_VERIFICATION", "yes"),
        ]))
        .unwrap();

        assert_eq!(config.port, 8080);
        assert_eq!(config.alexa_skill_id.as_deref(), Some("amzn1.ask.skill.1"));
        assert!(config.skip_signature_verification);
    }

    #[test]
    fn missing_required_variable_is_an_error() {
        let config = Config::from_env_source(source(&[
            ("NAVIDROME_URL", "https://n.example"),
            ("NAVIDROME_USER", "u"),
        ]));

        assert!(matches!(
            config,
            Err(ConfigError::Missing("NAVIDROME_PASSWORD"))
        ));
    }

    #[test]
    fn invalid_port_is_an_error() {
        let mut entries = required().to_vec();
        entries.push(("PORT", "not-a-port"));

        let config = Config::from_env_source(source(&entries));

        assert!(matches!(config, Err(ConfigError::InvalidPort(_))));
    }

    #[test]
    fn out_of_range_port_is_an_error() {
        let mut entries = required().to_vec();
        entries.push(("PORT", "65536"));

        let config = Config::from_env_source(source(&entries));

        assert!(matches!(config, Err(ConfigError::InvalidPort(_))));
    }

    #[test]
    fn boolean_values_are_case_insensitive() {
        for value in ["true", "TRUE", "1", "Yes"] {
            assert!(parse_bool(value).unwrap(), "value: {value}");
        }
        for value in ["false", "FALSE", "0", "No"] {
            assert!(!parse_bool(value).unwrap(), "value: {value}");
        }
    }

    #[test]
    fn invalid_boolean_is_an_error() {
        let mut entries = required().to_vec();
        entries.push(("SKIP_SIGNATURE_VERIFICATION", "maybe"));

        let config = Config::from_env_source(source(&entries));

        assert!(matches!(config, Err(ConfigError::InvalidBool(_))));
    }
}
