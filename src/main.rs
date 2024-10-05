
use std::{time::Duration, env::VarError};

use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use clap::{command, arg, value_parser};

const DEFAULT_REFRESH_TIMEOUT: u64 = 15 * 60;

async fn get_ip(client: &mut Client) -> Result<String> {
    let ip = client.get("https://api.ipify.org")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    eprintln!("{}", ip);
    Ok(ip)
}

fn get_string_env<S: AsRef<str>>(name: S) -> Option<String> {
    let name_ref = name.as_ref();
    match std::env::var(name_ref) {
        Ok(string) => Some(string),
        Err(VarError::NotPresent) => None,
        Err(VarError::NotUnicode(_)) => {
            log::warn!("The environment variable {} could not be decoded to unicode", name_ref);
            None
        },
    }
}

fn split_domain<S: Into<String>>(domain: S) -> (String, String) {
    let domain = domain.into();
    let parts: Vec<&str> = domain.split(".").collect();
    let sz = parts.len();
    let mut iter = parts.iter().cloned();
    (
        iter.by_ref().take(sz-2).collect::<Vec<&str>>().join("."),
        iter.collect::<Vec<&str>>().join(".")
    )
}

#[tokio::main]
async fn main() -> Result<()> {

    env_logger::init();

    let matches = command!()
        .arg(arg!(<domain> "The DNS zone that should be updated (with subdomain)"))
        .arg(arg!(--username [STRING] "The username to log in with"))
        .arg(arg!(--password [STRING] "The password to log in with"))
        .arg(arg!(--token [STRING] "Use this token insted of requesting one"))
        .arg(arg!(--refresh_timeout <SECS> "Time to wait between updates").value_parser(value_parser!(u64)))
        .get_matches();

    let mut username = matches.get_one::<String>("username").cloned();
    let mut password = matches.get_one::<String>("password").cloned();
    let mut token = matches.get_one::<String>("token").cloned();
    let refresh_timeout = matches.get_one::<u64>("refresh_timeout").cloned().unwrap_or(DEFAULT_REFRESH_TIMEOUT);
    let full_domain = matches.get_one::<String>("domain").unwrap();
    let (subdomain, domain) = split_domain(full_domain);

    // OpenProvider.nl lists the root domain as a subdomain
    let subdomain = if subdomain == "" { domain.clone() } else { subdomain };

    username = username.or(get_string_env("OPENPROVIDER_USERNAME"));
    password = password.or(get_string_env("OPENPROVIDER_PASSWORD"));
    token = token.or(get_string_env("OPENPROVIDER_TOKEN"));

    let mut client = Client::new();

    let mut op_client = openprovider::Builder::new().build();

    if token.is_some() {
        op_client.set_token(token.unwrap());
    }

    if !op_client.has_token() {
        let token = op_client.login(username.unwrap(), password.unwrap()).await?;
        op_client.set_token(token);
    }

    let mut orig_record = None;
    for record in op_client.list_records(&domain).await? {
        if record.ty == openprovider::RecordType::A && record.name == subdomain {
            let mut new_record = record.clone();
            if record.name == domain {
                new_record.name = "".to_string();
            }
            orig_record = Some(new_record);
            break;
        }
    }

    loop {

        let new_ip = get_ip(&mut client).await?;

        log::info!("Current IP {}", new_ip);

        match &mut orig_record {
            None => log::error!("A-record '{}' not found in domain {}", subdomain, domain),
            Some(record) => {
                if record.value == new_ip {
                    log::info!("IP address for {} is up-to-date", full_domain);
                } else {
                    let mut new_record = record.clone();
                    new_record.value = new_ip;
                    op_client.set_record(&domain, &record, &new_record).await?;
                    orig_record = Some(new_record);
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(refresh_timeout)).await;

    }

}
