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

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::runtime::Handle;
use tracing::debug;
use url::Url;

#[derive(Debug, Serialize)]
pub struct RelatedLink {
    pub url: String,
    pub discussion_url: String,
    pub description: String,
    pub upvotes: usize,
    pub comments: usize,
    pub date: String,
}

type RelatedLinkResult = Result<Vec<RelatedLink>, Box<dyn Error>>;

#[derive(Debug, Deserialize)]
pub struct HnRelatedLinkSearch {
    pub hits: Vec<HnRelatedLinkSearchHits>,
}

#[derive(Debug, Deserialize)]
pub struct HnRelatedLinkSearchHits {
    pub created_at: String,
    pub num_comments: usize,
    pub points: usize,
    pub url: String,
    pub title: String,
    #[serde(alias = "objectID")]
    pub object_id: String,
}

pub async fn hackernews(site: &str, _handle: Handle) -> RelatedLinkResult {
    let client = reqwest::Client::new();
    let res = client
        .get(format!(
            "https://hn.algolia.com/api/v1/search?query={site}&restrictSearchableAttributes=url"
        ))
        .send()
        .await?;

    if res.status() != 200 {
        return Err(format!("error status: {}", res.status()).into());
    }

    let json = res.text().await?;
    let res_json = serde_json::from_str::<HnRelatedLinkSearch>(&json[..])?;

    let mut related = vec![];
    for link in res_json.hits {
        if !link.url.contains(site) {
            // Algolia sometimes returns 'close' search results for entirely
            // different domains.
            debug!("{} doesn't contain {site}; skipping", link.url);
            continue;
        }

        let discussion_url = format!("https://news.ycombinator.com/item?id={}", link.object_id);

        if link.num_comments == 0 {
            debug!("no comments for {discussion_url}; skipping");
        }

        if check_link(&link.url).await {
            related.push(RelatedLink {
                url: link.url,
                discussion_url,
                upvotes: link.points,
                comments: link.num_comments,
                description: link.title,
                date: link.created_at,
            });
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    related.sort_by(|x, y| y.upvotes.cmp(&x.upvotes));

    Ok(related)
}

pub async fn lobsters(site: &str, _handle: Handle) -> RelatedLinkResult {
    let url = Url::parse(site)?;

    // Lobsters only has a domain selector for search; using a URL is
    // unreliable without using the selector and doesn't work at all with
    // the domain selector.
    let Some(host) = url.host_str() else {
        return Err("unable to get hostname from url".into());
    };

    let client = reqwest::Client::new();
    let res = client
        .get(format!(
            "https://lobste.rs/search?q=domain:{host}&what=stories&order=score",
        ))
        .send()
        .await?;

    if res.status() != 200 {
        return Err(format!("error status: {}", res.status()).into());
    }

    let html = res.text().await?;

    let story_re = Regex::new(
        r#"(?smx)^<div\ class="story_liner\ h-entry">$
                                 .*?
                                 <div\ class="score">(.*?)</div>
                                 .*?
                                 <a\ class="u-url"\ href="(.*?)"\ .*?>(.*?)</a>
                                 .*?
                                 <span\ title="(.*?)">
                                 .*?
                                 <a\ role="heading"\ aria-level="2"\ href="(.*?)">
                                 .*?
                                 (\d+)\ comments
                                 .*?
                                 ^</div>$"#,
    )
    .unwrap();

    let mut related = vec![];

    for (_, [score, url, description, date, discussion, comments]) in
        story_re.captures_iter(&html).map(|c| c.extract())
    {
        let url = String::from(url);

        // Because we can't (reliably) search by URL, make sure the
        // submitted URL is contained in the site link from lobsters
        if !url.contains(site) {
            debug!("{url} doesn't contain {site}; skipping");
            continue;
        }

        if check_link(&url).await {
            let score = score.parse().unwrap_or(0);
            let comments = comments.parse().unwrap_or(0);

            if comments == 0 {
                debug!("no comments for {discussion}; skipping");
                continue;
            }

            related.push(RelatedLink {
                url,
                upvotes: score,
                comments,
                description: String::from(description),
                date: String::from(date),
                discussion_url: format!("https://lobste.rs{discussion}"),
            });
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    related.sort_by(|x, y| y.upvotes.cmp(&x.upvotes));

    Ok(related)
}

pub async fn check_link(url: &String) -> bool {
    let client = reqwest::Client::new();

    match client.get(url).send().await {
        Ok(res) => {
            debug!("check_link HTTP status code: {}", res.status());
            res.status() == 200
        }
        Err(e) => {
            debug!("check_link error: {e:?}");
            false
        }
    }
}
