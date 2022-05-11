use std::cell::RefCell;
use std::convert::TryFrom;
use std::fs;
use std::rc::Rc;

use anyhow::Result;
use clap::ArgMatches;
use deno_core::error::AnyError;
use deno_core::{
    op, Extension, FsModuleLoader, JsRuntime, ModuleSpecifier, OpState, RuntimeOptions,
};

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};

// TODO: Can we plumb the async wasm_source directly into instantiateStreaming without await?
const WASM_LOADER: &str = r#"
var importObject = {
    phylum: {
        simple: () => Deno.core.opSync("simple")
    }
};

async function load() {
    const wasmCode = new Uint8Array(await Deno.core.opAsync("wasm_source"));
    const wasm = await WebAssembly.instantiateStreaming(wasmCode, importObject);
    const main = wasm.instance.exports.main;
    Deno.core.print(main().toString());
}
load();
"#;

#[op]
pub async fn projects(state: Rc<RefCell<OpState>>) -> Result<Vec<String>, AnyError> {
    let mut state = state.borrow_mut();
    let api = state.borrow_mut::<PhylumApi>();

    let response = api.get_user_settings().await?;
    let names = response.projects.keys().cloned().collect();

    Ok(names)
}

#[op]
pub async fn wasm_source(state: Rc<RefCell<OpState>>) -> Result<Vec<u8>, AnyError> {
    let state = state.borrow();
    let wasm = state
        .try_borrow::<WasmSource>()
        .cloned()
        .unwrap_or_default();
    Ok(wasm.0)
}

#[op]
pub fn simple() -> Result<u8, AnyError> {
    Ok(90)
}

pub async fn handle_deno(api: PhylumApi, matches: &ArgMatches) -> CommandResult {
    let extension = Extension::builder()
        .ops(vec![projects::decl(), wasm_source::decl(), simple::decl()])
        .build();

    let mut runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(FsModuleLoader)),
        extensions: vec![extension],
        ..Default::default()
    });

    // Load JS script.
    let exec_path = matches.value_of("file").unwrap();
    let exec = if exec_path.ends_with(".wasm") {
        // Wrap wasm executables with JS loader.
        let src = fs::read(exec_path)?;
        runtime.op_state().borrow_mut().put(WasmSource(src));
        WASM_LOADER.to_string()
    } else {
        fs::read_to_string(exec_path)?
    };

    runtime.op_state().borrow_mut().put(api);

    // NOTE: This is how module loading would work instead of `execute_script`
    // let specifier = ModuleSpecifier::parse("file:main.js")?;
    // let module = runtime.load_main_module(&specifier, Some(exec)).await?;
    // let _ = runtime.mod_evaluate(module);

    runtime.execute_script(exec_path, &exec)?;
    runtime.run_event_loop(false).await?;

    Ok(ExitCode::Ok.into())
}

/// Wasm binary data.
#[derive(Default, Clone)]
struct WasmSource(Vec<u8>);
