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

use std::{env, str};

use actix_web::{
    get, http::header::ContentType, post, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder, Result,
};
use minijinja::{context, Environment};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use url::Url;

use tenkbclub::{
    analyzer::analyzer,
    config::{Config, LogLevel},
    database::{
        cast_vote, generate_id, get_related, get_site_count, get_site_url, get_sites, get_votes,
        init_db, submit_site, Pool,
    },
    error::{HtmlError, JsonError},
    get_client_ip, get_page_links, SortOptions,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = Config::load(&env::var("TENKB_CONFIG").unwrap_or("/etc/tenkb.json".into())[..])?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(match config.log_level {
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        })
        .without_time()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Could not set default global tracing subscriber");

    let pool = init_db(&config.database_path);

    let analyzer_pool = pool.clone();
    let analyzer_config = config.clone();
    tokio::task::spawn(async move {
        loop {
            match analyzer(&analyzer_pool, &analyzer_config).await {
                Ok(_) => error!("analyzer exited unexpectedly with Ok. Restarting."),
                Err(e) => error!("analyzer exited with error: {e:?}. Restarting."),
            }
        }
    });

    let mut env = Environment::new();
    env.set_loader(minijinja::path_loader(config.template_path));

    HttpServer::new(move || {
        let app = App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(env.clone()))
            .service(index)
            .service(submit)
            .service(submithtml)
            .service(related)
            .service(id)
            .service(vote)
            .service(votes);

        if cfg!(debug_assertions) {
            app.service(css).service(js)
        } else {
            app
        }
    })
    .bind((config.listen_addr, config.listen_port))?
    .run()
    .await
}

#[get("/10kb.css")]
async fn css() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType(mime::TEXT_CSS))
        .body(include_str!("/home/marcusb/code/10kbclub/static/10kb.css"))
}

#[get("/10kb.js")]
async fn js() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType(mime::TEXT_JAVASCRIPT))
        .body(include_str!("/home/marcusb/code/10kbclub/static/10kb.js"))
}

#[get("/submit.html")]
#[allow(clippy::needless_lifetimes)]
async fn submithtml<'a>(template: web::Data<Environment<'a>>) -> Result<impl Responder, HtmlError> {
    Ok(HttpResponse::Ok()
        .content_type(ContentType(mime::TEXT_HTML))
        .body(
            template
                .get_template("submit.html")?
                .render(context!(title => format!("Submit a site")))?,
        ))
}

#[derive(Deserialize)]
struct ViewRequest {
    sortby: Option<SortOptions>,
    paginate: Option<usize>,
    page: Option<usize>,
}

#[get("/")]
#[allow(clippy::needless_lifetimes)]
async fn index<'a>(
    query: web::Query<ViewRequest>,
    template: web::Data<Environment<'a>>,
    pool: web::Data<Pool>,
    req: HttpRequest,
) -> Result<impl Responder, HtmlError> {
    let page = match query.page {
        Some(0) | None => 1,
        Some(page) => page,
    };
    let sortby = query.sortby.unwrap_or(SortOptions::Votes);
    let paginate = query.paginate.unwrap_or(25);
    let offset = paginate * (page - 1);
    let client_ip = get_client_ip(&req)?;

    info!("Generating index for {client_ip}");

    let tmp = pool.clone();
    let count = web::block(move || get_site_count(&tmp)).await??;

    let (page_links, prev_link, next_link) =
        get_page_links(page, count as f32, paginate as f32, sortby);

    let sites = web::block(move || get_sites(&pool, sortby, offset, paginate)).await??;

    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(
        template.get_template("index.html")?.render(context!(
            sites => sites,
            page_links => page_links,
            next_link => next_link,
            prev_link => prev_link,
        ))?,
    ))
}

#[get("/related/{site}/")]
#[allow(clippy::needless_lifetimes)]
async fn related<'a>(
    path: web::Path<u32>,
    template: web::Data<Environment<'a>>,
    pool: web::Data<Pool>,
    req: HttpRequest,
) -> Result<impl Responder, HtmlError> {
    let site = path.into_inner();
    let client_ip = get_client_ip(&req)?;
    info!("getting related links for '{site}' {client_ip}");

    let related = get_related(&pool, site)?;
    let url = get_site_url(&pool, site)?;

    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(
        template.get_template("related.html")?.render(context!(
            url => url,
            related => related,
            title => format!("Related links for {url}"),
        ))?,
    ))
}

#[derive(Debug, Deserialize)]
struct SubmitRequest {
    site: String,
}

#[post("/dosubmit/")]
#[allow(clippy::needless_lifetimes)]
async fn submit<'a>(
    query: web::Form<SubmitRequest>,
    template: web::Data<Environment<'a>>,
    pool: web::Data<Pool>,
    req: HttpRequest,
) -> Result<impl Responder, HtmlError> {
    let client_ip = get_client_ip(&req)?;
    let site = query.site.clone();

    Url::parse(&site[..])?;

    info!("adding '{site}' to submission queue for {client_ip}");
    submit_site(pool, site.clone())?;

    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(
        template.get_template("submitted.html")?.render(context!(
            title => format!("Site Submitted: {site}"),
            site => site,
        ))?,
    ))
}

#[derive(Serialize)]
struct IdResponse {
    code: usize,
    status: String,
    voter_id: String,
}

#[post("/id/")]
async fn id(pool: web::Data<Pool>, req: HttpRequest) -> Result<impl Responder, JsonError> {
    let mut response = IdResponse {
        code: 200,
        status: String::from("OK"),
        voter_id: String::from(""),
    };

    let client_ip = get_client_ip(&req)?;

    let mut rand_bytes = [0u8; 32];
    thread_rng().fill(&mut rand_bytes);

    let id = hex::encode(rand_bytes);
    response.voter_id = id.clone();

    info!("Generating new ID '{id}' for client {client_ip}");

    web::block(move || generate_id(pool, id)).await??;
    Ok(web::Json(response))
}

#[derive(Deserialize)]
struct VoteRequest {
    voter_id: String,
    site_id: u32,
    vote: isize,
}

#[derive(Serialize)]
struct VoteResponse {
    code: usize,
    status: String,
}

#[post("/vote/")]
async fn vote(
    data: web::Form<VoteRequest>,
    pool: web::Data<Pool>,
    req: HttpRequest,
) -> Result<impl Responder, JsonError> {
    let voter_id = data.voter_id.clone();
    let site_id = data.site_id;
    let vote = data.vote;

    let response = VoteResponse {
        code: 200,
        status: String::from("OK"),
    };

    if !(0..=1).contains(&vote) {
        return Err("invalid vote".into());
    }

    let client_ip = get_client_ip(&req)?;

    info!(
        "casting vote '{vote}' for commenter: '{voter_id}' for site {site_id} from ip {client_ip}"
    );

    web::block(move || cast_vote(pool, voter_id, site_id, vote)).await??;

    Ok(web::Json(response))
}

#[derive(Deserialize)]
struct VotesRequest {
    voter_id: String,
    site_ids: String,
}

#[derive(Serialize)]
struct VotesResponse {
    code: usize,
    status: String,
    site_ids: Vec<u32>,
}

#[post("/votes/")]
async fn votes(
    data: web::Form<VotesRequest>,
    pool: web::Data<Pool>,
    req: HttpRequest,
) -> Result<impl Responder, JsonError> {
    let voter_id = data.voter_id.clone();
    let site_ids = data
        .site_ids
        .split(",")
        .filter_map(|s| if let Ok(n) = s.parse() { Some(n) } else { None })
        .collect::<Vec<u32>>();

    let mut response = VotesResponse {
        code: 200,
        status: String::from("OK"),
        site_ids: vec![],
    };

    let client_ip = get_client_ip(&req)?;

    info!("getting votes for '{voter_id}' from ip {client_ip}");

    let sites = web::block(move || get_votes(pool, voter_id)).await??;

    for site in sites {
        if site_ids.contains(&site) {
            response.site_ids.push(site);
        }
    }

    Ok(web::Json(response))
}
