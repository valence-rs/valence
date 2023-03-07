use std::path::PathBuf;
use std::{collections::BTreeMap, net::SocketAddr};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::MetaPacket;

#[derive(Serialize, Deserialize)]
pub struct ApplicationConfig {
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    max_connections: Option<usize>,
    filter: Option<String>,
    selected_packets: Option<BTreeMap<MetaPacket, bool>>,
    // packets: Option<Vec<String>>,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:25565".parse().unwrap(),
            client_addr: "127.0.0.1:25566".parse().unwrap(),
            max_connections: None,
            filter: None,
            selected_packets: None,
        }
    }
}

impl ApplicationConfig {
    pub fn load() -> ApplicationConfig {
        let config_dir = match get_or_create_project_dirs() {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Could not find config directory: {}", e);
                return ApplicationConfig::default();
            }
        };

        let config_file = config_dir.join("config.toml");

        if config_file.exists() {
            let config = match std::fs::read_to_string(config_file) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Could not read config file: {}", e);
                    return ApplicationConfig::default();
                }
            };

            match toml::from_str(&config) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Could not parse config file: {}", e);
                    ApplicationConfig::default()
                }
            }
        } else {
            let config = toml::to_string(&ApplicationConfig::default()).unwrap();
            match std::fs::write(config_file, config) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Could not write config file: {}", e);
                }
            }
            ApplicationConfig::default()
        }
    }

    pub fn server_addr(&self) -> SocketAddr {
        self.server_addr
    }

    pub fn client_addr(&self) -> SocketAddr {
        self.client_addr
    }

    pub fn max_connections(&self) -> Option<usize> {
        self.max_connections
    }

    pub fn filter(&self) -> &Option<String> {
        &self.filter
    }

    pub fn selected_packets(&self) -> &Option<BTreeMap<MetaPacket, bool>> {
        &self.selected_packets
    }

    pub fn set_server_addr(&mut self, addr: SocketAddr) {
        self.server_addr = addr;
    }

    pub fn set_client_addr(&mut self, addr: SocketAddr) {
        self.client_addr = addr;
    }

    pub fn set_max_connections(&mut self, max: Option<usize>) {
        self.max_connections = max;
    }

    pub fn set_filter(&mut self, filter: Option<String>) {
        self.filter = filter;
    }

    pub fn set_selected_packets(&mut self, packets: BTreeMap<MetaPacket, bool>) {
        self.selected_packets = Some(packets);
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = match get_or_create_project_dirs() {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Could not find config directory: {}", e);
                return Ok(());
            }
        };

        let config_file = config_dir.join("config.toml");

        let config = toml::to_string(&self).unwrap();
        std::fs::write(config_file, config).unwrap();
        Ok(())
    }
}

fn get_or_create_project_dirs() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "valence", "inspector") {
        // check if the directory exists, if not create it
        if !proj_dirs.config_dir().exists() {
            std::fs::create_dir_all(proj_dirs.config_dir())?;
        }

        Ok(proj_dirs.config_dir().to_owned())
    } else {
        Err("Could not find project directories".into())
    }
}
