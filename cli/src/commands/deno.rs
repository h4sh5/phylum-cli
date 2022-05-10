use std::cell::RefCell;
use std::convert::TryFrom;
use std::fs;
use std::rc::Rc;

use clap::ArgMatches;
use deno_core::error::AnyError;
use deno_core::{op, Extension, JsRuntime, OpState, RuntimeOptions};

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};

#[op]
pub async fn projects(state: Rc<RefCell<OpState>>) -> Result<Vec<String>, AnyError> {
    let mut state = state.borrow_mut();
    let api = state.borrow_mut::<PhylumApi>();

    let response = api.get_user_settings().await?;
    let names = response.projects.keys().cloned().collect();

    Ok(names)
}

pub async fn handle_deno(api: PhylumApi, matches: &ArgMatches) -> CommandResult {
    let exec_path = matches.value_of("file").unwrap();
    let exec = fs::read_to_string(exec_path)?;

    let extension = Extension::builder().ops(vec![projects::decl()]).build();

    let mut runtime = JsRuntime::new(RuntimeOptions {
        // extensions: vec![extension],
        ..Default::default()
    });

    runtime.op_state().borrow_mut().put(api);

    runtime.execute_script(exec_path, &exec)?;
    runtime.run_event_loop(false).await?;

    Ok(ExitCode::Ok.into())
}
