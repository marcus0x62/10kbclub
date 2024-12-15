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

use crate::config::Config;
use reqwest::header::{HeaderMap, HeaderName};
use serde::Deserialize;
use std::{collections::HashMap, error::Error};
use tokio::runtime::Handle;
use tracing::{debug, info};

#[derive(Debug)]
pub struct UrlScan {
    pub size: f64,
    pub acceptable: bool,
}

type UrlScanResult = Result<UrlScan, Box<dyn Error>>;

#[derive(Debug, Deserialize)]
struct UrlScanSubmit {
    result: UrlScanSubmitResult,
    success: bool,
}

#[derive(Debug, Deserialize)]
struct UrlScanSubmitResult {
    uuid: String,
}

#[derive(Debug, Deserialize)]
struct UrlScanReport {
    result: UrlScanReportResult,
    success: bool,
}

#[derive(Debug, Deserialize)]
struct UrlScanReportResult {
    scan: UrlScanReportResultScan,
}

#[derive(Debug, Deserialize)]
struct UrlScanReportResultScan {
    stats: UrlScanReportResultScanStats,
    verdicts: UrlScanReportScanVerdicts,
}

#[derive(Debug, Deserialize)]
struct UrlScanReportResultScanStats {
    requests: UrlScanReportResultScanStatsRequests,
}

#[derive(Debug, Deserialize)]
struct UrlScanReportResultScanStatsRequests {
    #[serde(alias = "transferSizeBytes")]
    transfer_size: u32,
}

#[derive(Debug, Deserialize)]
struct UrlScanReportScanVerdicts {
    overall: UrlScanReportVerdictsOverall,
}

#[derive(Debug, Deserialize)]
struct UrlScanReportVerdictsOverall {
    malicious: bool,
}

pub async fn urlscan(host: &str, _handle: Handle, config: &Config) -> UrlScanResult {
    let mut body = HashMap::new();
    let mut headers = HeaderMap::new();

    body.insert("url", host);

    let auth_header = format!("Bearer {}", config.cloudflare_api_token).parse()?;
    headers.insert(HeaderName::from_static("authorization"), auth_header);

    let client = reqwest::Client::new();
    let res = client
        .post(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/urlscanner/scan",
            config.cloudflare_account,
        ))
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    if res.status() != 200 {
        return Err(format!("error status: {}", res.status()).into());
    }

    let json = res.text().await?;
    let res_json = serde_json::from_str::<UrlScanSubmit>(&json[..])?;

    if !res_json.success {
        return Err(format!("error submitting {host} to cloudflare").into());
    }

    let scan_id = res_json.result.uuid;

    debug!("got uuid: {scan_id}");

    for _ in 0..3 {
        debug!("sleeping...");
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;

        let mut headers = HeaderMap::new();
        let auth_header = format!("Bearer {}", config.cloudflare_api_token).parse()?;
        headers.insert(HeaderName::from_static("authorization"), auth_header);

        let res = client
            .get(format!(
                "https://api.cloudflare.com/client/v4/accounts/{}/urlscanner/scan/{scan_id}",
                config.cloudflare_account
            ))
            .headers(headers)
            .send()
            .await?;

        match res.status().into() {
            200 => {}
            202 => continue,
            _ => return Err(format!("error status: {}", res.status()).into()),
        }

        let json = res.text().await?;
        let res_json = serde_json::from_str::<UrlScanReport>(&json[..])?;

        if !res_json.success {
            return Err(format!("error submitting {host} to cloudflare").into());
        }

        let acceptable_size =
            res_json.result.scan.stats.requests.transfer_size <= SIZE_LIMIT as u32;
        if !acceptable_size {
            info!(
                "{host} exceeds {SIZE_LIMIT}: {}",
                res_json.result.scan.stats.requests.transfer_size
            );
        }

        if res_json.result.scan.verdicts.overall.malicious {
            info!("{host} is malicious!");
        }

        return Ok(UrlScan {
            size: res_json.result.scan.stats.requests.transfer_size as f64,
            acceptable: acceptable_size && !res_json.result.scan.verdicts.overall.malicious,
        });
    }

    Err("unknown error".into())
}

const SIZE_LIMIT: usize = 10_240;
