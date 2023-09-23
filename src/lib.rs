//! # AWS Credentials Manipulation Library
//!
//! This library provides a convenient way to load, modify, and save AWS credentials stored in the standard AWS credentials file format.
//! With this library, you can easily manage profiles and their associated credentials without having to manually edit the credentials file.
//!
//! ## Example Usage
//! ```rust
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     use aws_cred::AWSCredentials;
//!     let mut credentials = AWSCredentials::load()?;
//!     credentials
//!         .with_profile("default")
//!         .set_access_key_id("ACCESS_KEY")
//!         .set_secret_access_key("SECRET_KEY");
//!     credentials.write()?;
//!     Ok(())
//! }
//! ```

use derive_builder::Builder;
use dirs::home_dir;
use std::{
    collections::HashMap,
    fmt,
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
};

#[cfg(feature = "async_std")]
use async_std::io::WriteExt;

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

#[cfg(feature = "rusoto")]
impl From<rusoto_sts::Credentials> for Credentials {
    fn from(credentials: rusoto_sts::Credentials) -> Self {
        Credentials {
            secret_access_key: credentials.secret_access_key,
            access_key_id: credentials.access_key_id,
            session_token: Some(credentials.session_token),
        }
    }
}

#[cfg(feature = "aws_sdk")]
impl TryFrom<aws_sdk_sts::types::Credentials> for Credentials {
    type Error = &'static str;

    fn try_from(credentials: aws_sdk_sts::types::Credentials) -> Result<Self, Self::Error> {
        Ok(Credentials {
            secret_access_key: credentials
                .secret_access_key
                .ok_or("Missing secret access key")?,
            access_key_id: credentials.access_key_id.ok_or("Missing access key id")?,
            session_token: credentials.session_token,
        })
    }
}

#[derive(Debug)]
pub enum Error {
    FileNotFound(String),
    FailedToParse,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::FileNotFound(path) => write!(f, "File not found: {}", path),
            Error::FailedToParse => write!(f, "Failed to parse"),
        }
    }
}

impl std::error::Error for Error {}

/// Contains a mapping of profiles to AWS credentials.
/// Provides methods to load from, and save to, the default AWS credentials file.
#[derive(Debug)]
pub struct AWSCredentials {
    file_path: String,
    credentials: HashMap<String, Credentials>,
}

impl AWSCredentials {
    /// Creates a new AWSCredentials instance.
    pub fn new<P: AsRef<Path>>(path: P) -> AWSCredentials {
        AWSCredentials {
            file_path: path.as_ref().to_str().unwrap().to_string(),
            credentials: HashMap::new(),
        }
    }

    /// Gets the credentials for the specified profile.
    pub fn get_profile(&self, profile: &str) -> Option<Credentials> {
        self.credentials.get(profile).cloned()
    }

    /// Gets a mutable reference to the credentials for the specified profile.
    pub fn get_profile_mut(&mut self, profile: &str) -> Option<&mut Credentials> {
        self.credentials.get_mut(profile)
    }

    /// Sets the credentials for the specified profile.
    pub fn set_profile(&mut self, profile: &str, credentials: &Credentials) {
        self.credentials
            .insert(profile.to_string(), credentials.clone());
    }

    /// Returns a profiles credentials setter, if the profile does not exist, it will be created.
    pub fn with_profile(&mut self, profile: &str) -> CredentialsSetter {
        if self.credentials.get(profile).is_none() {
            self.credentials
                .insert(profile.to_string(), Credentials::default());
        }
        CredentialsSetter::new(self, profile)
    }

    /// Checks if the specified profile exists.
    pub fn exists(&self, profile: &str) -> bool {
        self.credentials.contains_key(profile)
    }

    /// Removes the specified profile.
    pub fn remove_profile(&mut self, profile: &str) -> Option<Credentials> {
        self.credentials.remove(profile)
    }

    /// Load credentials from the default AWS credentials file location (`~/.aws/credentials`).
    pub fn load() -> Result<AWSCredentials, Error> {
        Self::load_from(&format!(
            "{}/.aws/credentials",
            home_dir().unwrap().to_str().unwrap()
        ))
    }

    /// Load credentials async from the default AWS credentials file location (`~/.aws/credentials`).
    #[cfg(feature = "async_std")]
    pub async fn load_async() -> Result<AWSCredentials, Error> {
        Self::load_from_async(&format!(
            "{}/.aws/credentials",
            home_dir().unwrap().to_str().unwrap()
        ))
        .await
    }

    /// Load credentials from the specified file path.
    pub fn load_from(file_path: &str) -> Result<AWSCredentials, Error> {
        let file = std::fs::read_to_string(file_path)
            .map_err(|_| Error::FileNotFound(file_path.to_string()))?;

        let credentials = Self::parse(file).map_err(|_| Error::FailedToParse)?;

        Ok(AWSCredentials {
            file_path: file_path.to_string(),
            credentials,
        })
    }

    /// Load credentials async from the specified file path.
    #[cfg(feature = "async_std")]
    pub async fn load_from_async(file_path: &str) -> Result<AWSCredentials, Error> {
        let file = async_std::fs::read_to_string(file_path)
            .await
            .map_err(|_| Error::FileNotFound(file_path.to_string()))?;

        let credentials = Self::parse(file).map_err(|_| Error::FailedToParse)?;

        Ok(AWSCredentials {
            file_path: file_path.to_string(),
            credentials,
        })
    }

    fn parse(data: String) -> Result<HashMap<String, Credentials>, Error> {
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

    /// Write credentials to the default AWS credentials file location (`~/.aws/credentials`).
    pub fn write(&self) -> Result<(), Error> {
        self.write_to(Path::new(&self.file_path))
    }

    /// Write credentials to the specified file path.
    pub fn write_to<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|_| Error::FileNotFound(self.file_path.to_string()))?;

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

    /// Write credentials async to the default AWS credentials file location (`~/.aws/credentials`).
    #[cfg(feature = "async_std")]
    pub async fn write_async(&self) -> Result<(), Error> {
        self.write_to_async(async_std::path::Path::new(&self.file_path))
            .await
    }

    #[cfg(feature = "async_std")]
    pub async fn write_to_async<P: AsRef<async_std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), Error> {
        let file = async_std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await
            .map_err(|_| Error::FileNotFound(self.file_path.to_string()))?;

        let mut writer = async_std::io::BufWriter::new(file);

        for (section, creds) in &self.credentials {
            writer
                .write(&format!("[{}]\n", section).into_bytes())
                .await
                .unwrap();
            writer
                .write(&format!("aws_access_key_id = {}\n", creds.access_key_id).into_bytes())
                .await
                .unwrap();
            writer
                .write(
                    &format!("aws_secret_access_key = {}\n", creds.secret_access_key).into_bytes(),
                )
                .await
                .unwrap();

            if let Some(session_token) = &creds.session_token {
                writer
                    .write(&format!("aws_session_token = {}\n", session_token).into_bytes())
                    .await
                    .unwrap();
            }

            writer.write(b"\n").await.unwrap();
        }
        writer.flush().await.unwrap();

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
    use super::AWSCredentials;
    use tempfile;

    #[test]
    fn can_load_credentials() {
        let temp_aws_credentials = tempfile::NamedTempFile::new().unwrap();
        // write credentials to the file use write_all
        std::fs::write(
            temp_aws_credentials.path(),
            r#"
[default]
aws_access_key_id = ACCESS_KEY
aws_secret_access_key = SECRET_KEY
"#,
        )
        .unwrap();

        let temp_aws_credentials_path = temp_aws_credentials.path().to_str().unwrap();
        let credentials = AWSCredentials::load_from(temp_aws_credentials_path).unwrap();
        let default_profile = credentials.get_profile("default").unwrap();
        assert_eq!(default_profile.access_key_id, "ACCESS_KEY");
        assert_eq!(default_profile.secret_access_key, "SECRET_KEY");
    }

    #[test]
    fn can_write_credentials() {
        let temp_aws_credentials = tempfile::NamedTempFile::new().unwrap();
        let temp_aws_credentials_path = temp_aws_credentials.path().to_str().unwrap();
        let mut credentials = AWSCredentials::new(temp_aws_credentials_path);
        credentials
            .with_profile("default")
            .set_access_key_id("ACCESS_KEY")
            .set_secret_access_key("SECRET_KEY")
            .set_session_token(Some("SESSION_TOKEN".to_string()));
        credentials.write().unwrap();

        let credentials = AWSCredentials::load_from(temp_aws_credentials_path).unwrap();
        let default_profile = credentials.get_profile("default").unwrap();
        assert_eq!(default_profile.access_key_id, "ACCESS_KEY");
        assert_eq!(default_profile.secret_access_key, "SECRET_KEY");
        assert_eq!(
            default_profile.session_token,
            Some("SESSION_TOKEN".to_string())
        );
    }

    #[cfg(feature = "async_std")]
    #[tokio::test]
    async fn can_load_credentials_async() {
        let temp_aws_credentials = tempfile::NamedTempFile::new().unwrap();
        // write credentials to the file use write_all
        std::fs::write(
            temp_aws_credentials.path(),
            r#"
[default]
aws_access_key_id = ACCESS_KEY
aws_secret_access_key = SECRET_KEY
"#,
        )
        .unwrap();

        let temp_aws_credentials_path = temp_aws_credentials.path().to_str().unwrap();
        let credentials = AWSCredentials::load_from_async(temp_aws_credentials_path)
            .await
            .unwrap();
        let default_profile = credentials.get_profile("default").unwrap();
        assert_eq!(default_profile.access_key_id, "ACCESS_KEY");
        assert_eq!(default_profile.secret_access_key, "SECRET_KEY");
    }

    #[cfg(feature = "async_std")]
    #[tokio::test]
    async fn can_write_credentials_async() {
        let temp_aws_credentials = tempfile::NamedTempFile::new().unwrap();
        let temp_aws_credentials_path = temp_aws_credentials.path().to_str().unwrap();
        let mut credentials = AWSCredentials::new(temp_aws_credentials_path);
        credentials
            .with_profile("default")
            .set_access_key_id("ACCESS_KEY")
            .set_secret_access_key("SECRET_KEY")
            .set_session_token(Some("SESSION_TOKEN".to_string()));
        credentials.write_async().await.unwrap();

        let credentials = AWSCredentials::load_from_async(temp_aws_credentials_path)
            .await
            .unwrap();
        let default_profile = credentials.get_profile("default").unwrap();
        assert_eq!(default_profile.access_key_id, "ACCESS_KEY");
        assert_eq!(default_profile.secret_access_key, "SECRET_KEY");
        assert_eq!(
            default_profile.session_token,
            Some("SESSION_TOKEN".to_string())
        );
    }
}
