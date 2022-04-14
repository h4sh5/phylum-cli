//! Subcommand `phylum group`.

use clap::ArgMatches;

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::print;
use crate::print_user_success;

/// Handle `phylum group` subcommand.
pub async fn handle_group(api: &mut PhylumApi, matches: &ArgMatches) -> CommandResult {
    if let Some(matches) = matches.subcommand_matches("create") {
        let group_name = matches.value_of("group_name").unwrap();
        let response = api.create_group(group_name).await?;
        print_user_success!("Successfully created group {}", response.group_name);
        Ok(ExitCode::Ok.into())
    } else {
        let pretty_print = !matches.is_present("json");
        let response = api.get_groups_list().await;

        // // TODO: Testing.
        // // My Awesome Group Name Is…__contact@christianduerr.com         __2022-12-03T08:15+02:00
        // let group = phylum_types::types::group::UserGroup {
        //     group_name: "My Awesome Group Nam Is Great".into(),
        //     created_at: chrono::offset::Utc::now(),
        //     last_modified: chrono::offset::Utc::now(),
        //     owner_email: "my-super-long-email-address@phylum.io".into(),
        //     is_admin: false,
        //     is_owner: false,
        // };
        // let response = Ok(phylum_types::types::group::ListUserGroupsResponse {
        //     groups: vec![group.clone(), group.clone(), group],
        // });

        print::print_response(&response, pretty_print, None);
        Ok(ExitCode::Ok.into())
    }
}
