use clap::ArgMatches;
use wasmtime::{Caller, Config, Engine, Linker, Module, Store};

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};
use crate::print;

pub async fn handle_wasm(api: PhylumApi, matches: &ArgMatches) -> CommandResult {
    let wasm = matches.value_of("FILE").unwrap();

    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, wasm)?;
    let mut store = Store::new(&engine, api);

    // Define functions to be called from WASM.
    let mut linker = Linker::new(&engine);
    linker.func_wrap0_async(
        "phylum",
        "print_projects",
        |caller: Caller<'_, PhylumApi>| {
            // TODO: Not sure if Box async move is actually idiomatic here.
            Box::new(async move {
                let response = caller.data().get_projects().await;
                print::print_response(&response, true, None);
            })
        },
    )?;

    let instance = linker.instantiate(&mut store, &module)?;

    // Call function from WASM plugin.
    let entry_point = instance
        .get_func(&mut store, "entry_point")
        .expect("`entry_point` was not an exported function");
    entry_point.call_async(&mut store, &[], &mut []).await?;

    Ok(ExitCode::Ok.into())
}
