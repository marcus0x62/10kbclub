// MIT License
//
// Copyright (c) 2024 Marcus Butler
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use serde::Deserialize;
use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};

#[derive(Clone, Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub template_path: PathBuf,

    #[serde(default = "log_level_default")]
    pub log_level: LogLevel,
    pub cloudflare_account: String,
    pub cloudflare_api_token: String,

    #[serde(default = "listen_addr_default")]
    pub listen_addr: IpAddr,
    #[serde(default = "listen_port_default")]
    pub listen_port: u16,
}

#[derive(Clone, Deserialize)]
pub enum LogLevel {
    Info,
    Warn,
    Debug,
    Trace,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&contents[..])?)
    }
}

fn log_level_default() -> LogLevel {
    LogLevel::Info
}

fn listen_addr_default() -> IpAddr {
    IpAddr::from(Ipv4Addr::LOCALHOST)
}

fn listen_port_default() -> u16 {
    3003
}
