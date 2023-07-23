//! # AWS Credentials Manipulation Library
//!
//! This library provides a convenient way to load, modify, and save AWS credentials stored in the standard AWS credentials file format.
//! With this library, you can easily manage profiles and their associated credentials without having to manually edit the credentials file.
//!
//! ## Example Usage
//! ```rust
//! let mut credentials = AWSCredentials::load()?;
//! credentials
//!     .with_profile("default")
//!     .set_access_key_id("ACCESS_KEY")
//!     .set_secret_access_key("SECRET_KEY");
//! credentials.write()?;
//! ```

use derive_builder::Builder;
use dirs::home_dir;
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    fs::OpenOptions,
    io::{BufWriter, Write},
};

/// Represents AWS credentials with fields for access and secret keys.
#[derive(Clone, Builder, Debug, Default)]
pub struct Credentials {
    pub secret_access_key: String,
    pub access_key_id: String,
    #[builder(setter(into, strip_option), default)]
    pub session_token: Option<String>,
}

impl Credentials {
    pub(crate) fn set_secret_access_key(&mut self, value: String) {
        self.secret_access_key = value;
    }

    pub(crate) fn set_access_key_id(&mut self, value: String) {
        self.access_key_id = value;
    }

    pub(crate) fn set_session_token(&mut self, value: Option<String>) {
        self.session_token = value;
    }
}

#[derive(Debug)]
pub enum Errors {
    FileNotFound(String),
    FailedToParse,
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Errors::FileNotFound(path) => write!(f, "File not found: {}", path),
            Errors::FailedToParse => write!(f, "Failed to parse"),
        }
    }
}

impl Error for Errors {}

/// Contains a mapping of profiles to AWS credentials.
/// Provides methods to load from, and save to, the default AWS credentials file.
#[derive(Debug)]
pub struct AWSCredentials {
    file_path: String,
    credentials: HashMap<String, Credentials>,
}

impl AWSCredentials {
    pub fn get_profile(&self, profile: &str) -> Option<Credentials> {
        self.credentials.get(profile).cloned()
    }

    pub fn get_profile_mut(&mut self, profile: &str) -> Option<&mut Credentials> {
        self.credentials.get_mut(profile)
    }

    pub fn set_profile(&mut self, profile: &str, credentials: &Credentials) {
        self.credentials
            .insert(profile.to_string(), credentials.clone());
    }

    pub fn with_profile(&mut self, profile: &str) -> CredentialsSetter {
        if self.credentials.get(profile).is_none() {
            self.credentials
                .insert(profile.to_string(), Credentials::default());
        }
        CredentialsSetter::new(self, profile)
    }

    pub fn load() -> Result<AWSCredentials, Errors> {
        Self::load_from(&format!(
            "{}/.aws/credentials",
            home_dir().unwrap().to_str().unwrap()
        ))
    }

    pub fn load_from(file_path: &str) -> Result<AWSCredentials, Errors> {
        let file = std::fs::read_to_string(file_path)
            .map_err(|_| Errors::FileNotFound(file_path.to_string()))?;

        let credentials = Self::parse(file).map_err(|_| Errors::FailedToParse)?;

        Ok(AWSCredentials {
            file_path: file_path.to_string(),
            credentials,
        })
    }

    fn parse(data: String) -> Result<HashMap<String, Credentials>, Errors> {
        let mut credentials_map = HashMap::new();

        let mut current_section = String::new();
        let mut current_builder: Option<CredentialsBuilder> = None;

        for line in data.lines() {
            let line = line.trim().to_string();

            if line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                if let Some(builder) = current_builder {
                    credentials_map.insert(current_section, builder.build().unwrap());
                };
                current_builder = Some(CredentialsBuilder::default());
                current_section = line[1..line.len() - 1].to_string();
            }

            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
            } else if !line.is_empty() && !line.starts_with('#') {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value = parts[1].trim();
                    if let Some(builder) = current_builder.as_mut() {
                        match key {
                            "aws_access_key_id" => {
                                builder.access_key_id(value.to_string());
                            }
                            "aws_secret_access_key" => {
                                builder.secret_access_key(value.to_string());
                            }
                            "aws_session_token" => {
                                builder.session_token(value.to_string());
                            }
                            _ => (),
                        }
                    }
                }
            }
        }

        if let Some(builder) = current_builder {
            credentials_map.insert(current_section, builder.build().unwrap());
        };

        Ok(credentials_map)
    }

    pub fn write(&self) -> Result<(), Errors> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.file_path)
            .map_err(|_| Errors::FileNotFound(self.file_path.to_string()))?;

        let mut writer = BufWriter::new(file);

        for (section, creds) in &self.credentials {
            writeln!(writer, "[{}]", section).unwrap();
            writeln!(writer, "aws_access_key_id = {}", creds.access_key_id).unwrap();
            writeln!(
                writer,
                "aws_secret_access_key = {}",
                creds.secret_access_key
            )
            .unwrap();

            if let Some(session_token) = &creds.session_token {
                writeln!(writer, "aws_session_token = {}", session_token).unwrap();
            }

            writeln!(writer).unwrap();
        }

        Ok(())
    }
}

/// A setter which could be used to set key-value pair in a specified section
pub struct CredentialsSetter<'a> {
    aws_credentials: &'a mut AWSCredentials,
    profile_name: String,
}

impl<'a> CredentialsSetter<'a> {
    fn new<V>(aws_credentials: &'a mut AWSCredentials, profile_name: V) -> CredentialsSetter<'a>
    where
        V: Into<String>,
    {
        CredentialsSetter {
            aws_credentials,
            profile_name: profile_name.into(),
        }
    }

    pub fn set_secret_access_key<V>(&'a mut self, value: V) -> &'a mut CredentialsSetter<'a>
    where
        V: Into<String>,
    {
        if let Some(credentials) = self.aws_credentials.get_profile_mut(&self.profile_name) {
            credentials.set_secret_access_key(value.into());
        };
        self
    }

    pub fn set_access_key_id<V>(&'a mut self, value: V) -> &'a mut CredentialsSetter<'a>
    where
        V: Into<String>,
    {
        if let Some(credentials) = self.aws_credentials.get_profile_mut(&self.profile_name) {
            credentials.set_access_key_id(value.into());
        };
        self
    }

    pub fn set_session_token<V>(&'a mut self, value: Option<V>) -> &'a mut CredentialsSetter<'a>
    where
        V: Into<String>,
    {
        if let Some(credentials) = self.aws_credentials.get_profile_mut(&self.profile_name) {
            credentials.set_session_token(value.map(Into::into));
        };
        self
    }

    pub fn clear_session_token<V>(&'a mut self) -> &'a mut CredentialsSetter<'a>
    where
        V: Into<String>,
    {
        if let Some(credentials) = self.aws_credentials.get_profile_mut(&self.profile_name) {
            credentials.set_session_token(None);
        };
        self
    }
}

#[cfg(test)]
mod test {
    use crate::AWSCredentials;

    #[test]
    fn can_load_credentials() {
        if let Ok(aws_credentials) = AWSCredentials::load() {
            aws_credentials.write().unwrap();
        }
    }
}
