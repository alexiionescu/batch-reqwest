use anyhow::Result;
use clap::Parser as _;
use itertools::Itertools;
use log::{debug, error, info};
use reqwest::{
    ClientBuilder,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue},
};
use std::{path::PathBuf, time::Duration};

mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();

    // CLI options are defined later in this file
    let cli = Cli::parse();

    let mut csv_rdr = csv::Reader::from_path(cli.hosts_csv)?;
    let csv_headers = csv_rdr.headers()?.clone();
    info!("CSV Columns {csv_headers:?}");

    let mut headers = HeaderMap::new();
    headers.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let body = if let Some(json_file) = cli.body {
        tokio::fs::read_to_string(json_file).await?
    } else {
        "{}".into()
    };

    let client = ClientBuilder::new()
        .connect_timeout(Duration::from_secs(cli.timeout))
        .default_headers(headers)
        .build()?;

    println!("URL\t{}", cli.json_paths.iter().join("\t"));

    for r in csv_rdr.records().filter_map(Result::ok) {
        debug!("Processing {r:?}");
        let url = utils::replace_vars_from_csv(&cli.url, &csv_headers, &r);
        let body = utils::replace_vars_from_csv(&body, &csv_headers, &r);
        if cli.dry_run {
            info!("URL: {url} -> {body}");
        } else {
            debug!("URL: {url} -> {body}");

            let mut req = client.request(cli.method.clone(), url.clone()).body(body);
            if let Some(u) = &cli.username {
                req = req.basic_auth(u, cli.password.as_ref());
            }

            match req.send().await {
                Ok(res) => {
                    if res.status().is_success() {
                        match res.text().await {
                            Ok(res_body) => {
                                match serde_json::from_str::<serde_json::Value>(&res_body) {
                                    Ok(res_value) => {
                                        if !cli.json_paths.is_empty() {
                                            let mut selector = jsonpath_lib::selector(&res_value);
                                            println!(
                                                "{url}\t{}",
                                                cli.json_paths
                                                    .iter()
                                                    .flat_map(
                                                        |path| selector(path).unwrap_or_default()
                                                    )
                                                    .join("\t")
                                            );
                                        } else {
                                            info!("{url} Res: {res_value}");
                                        }
                                    }
                                    Err(err) => {
                                        error!(
                                            "{url} Error parsing response body: {err}\n\t{res_body}"
                                        )
                                    }
                                }
                            }
                            Err(err) => error!("{url} Error wait response body: {err}"),
                        }
                    } else {
                        error!("{url} Response Error {}", res.status());
                    }
                }
                Err(err) => error!("{url} Error send request: {err}"),
            }
        }
    }
    Ok(())
}

#[derive(clap::Parser)]
#[clap(trailing_var_arg = true)]
pub struct Cli {
    #[clap(
        long,
        help = "CSV where the first 'Host' column contains hosts to connect to"
    )]
    hosts_csv: PathBuf,

    #[clap(
        long,
        help = "url template, use <%CSV_COLUMN_NAME%> for variables replace"
    )]
    url: String,

    #[clap(long, short)]
    username: Option<String>,

    #[clap(long)]
    password: Option<String>,

    #[clap(long)]
    body: Option<PathBuf>,

    #[clap(long, default_value_t = reqwest::Method::POST)]
    method: reqwest::Method,

    #[clap(long, help = "connection timeout in seconds", default_value_t = 3)]
    timeout: u64,

    #[clap(num_args(1..), help = "json paths to extract from response to tab separated output")]
    json_paths: Vec<String>,

    #[clap(long)]
    dry_run: bool,
}
