use serde::Deserialize;
use std::{fs::File, io::Read, net::IpAddr};

#[derive(Deserialize, Debug, Clone)]
pub struct NodeConfig {
    pub id: String,
    pub node_type: i32,
    pub ip: IpAddr,
    pub k: u32,
    pub traffic_type: usize,
    pub topic_type: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MomTaskConfig {
    pub id: u8,
    pub priority: i32,
    pub in_endpoint: String,
    pub out_endpoint: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub nodes: Vec<NodeConfig>,
    pub tasks: Vec<MomTaskConfig>,
    pub registration_ip: String,
    pub load_ip: String,
    pub n_azure: i32,
    pub max_node_usage: f64,
}

impl Config {
    pub fn load_config_from_file(filename: &str) -> anyhow::Result<Config> {
        let mut file = File::open(filename)?;
        let mut buf = String::new();

        file.read_to_string(&mut buf)?;

        let mut config: Config = toml::from_str(&buf)?;

        for node in &config.nodes {
            if node.node_type == 1 {
                config.n_azure += 1;
            }
        }

        Ok(config)
    }
}
