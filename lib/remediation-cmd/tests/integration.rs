use phylum_cli::api::PhylumApi;
use phylum_cli::config::*;
use remediation_cmd::PhylumApiExt;

use anyhow::Result;

#[tokio::test]
async fn simple_request_yarn() {
    simplelog::SimpleLogger::init(log::LevelFilter::Debug, Default::default()).ok();

    let config_path = get_home_settings_path().expect("Couldn't read home settings path");
    let mut config = read_configuration(&config_path).expect("Couldn't read configuration");

    println!("{:?}", config.auth_info);

    let api = PhylumApi::new(
        &mut config.auth_info,
        &config.connection.uri,
        Some(1024),
        true,
    )
    .await
    .expect("Couldn't create PhylumApi");

    let remediations = api
        .remediation_yarn(
            include_str!("fixtures/yarn02/yarn.lock"),
            include_str!("fixtures/yarn02/package.json"),
        )
        .await
        .expect("Couldn't query for Yarn remediations");

    println!("{:?}", remediations);
}
