use std::cell::RefCell;
use std::convert::TryFrom;
use std::fs;
use std::pin::Pin;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use clap::ArgMatches;
use deno_ast::{MediaType, ParseParams, SourceTextInfo};
use deno_core::error::AnyError;
use deno_core::{
    op, Extension, JsRuntime, ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier,
    ModuleType, OpState, RuntimeOptions,
};
use serde::Serialize;

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

#[derive(Serialize)]
pub struct Project {
    pub name: String,
    pub number: u8,
}

#[op]
pub async fn projects(state: Rc<RefCell<OpState>>) -> Result<Vec<Project>, AnyError> {
    let mut state = state.borrow_mut();
    let api = state.borrow_mut::<PhylumApi>();

    let response = api.get_user_settings().await?;
    let names = response
        .projects
        .keys()
        .cloned()
        .map(|name| Project { name, number: 3 })
        .collect();

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
        module_loader: Some(Rc::new(TypescriptModuleLoader)),
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
    let specifier = deno_core::resolve_path(exec_path)?;
    let module = runtime.load_main_module(&specifier, None).await?;
    let _ = runtime.mod_evaluate(module);

    runtime.run_event_loop(false).await?;

    Ok(ExitCode::Ok.into())
}

/// Wasm binary data.
#[derive(Default, Clone)]
struct WasmSource(Vec<u8>);

/// Blatantly stolen from
/// https://github.com/denoland/deno/blob/main/core/examples/ts_module_loader.rs.
struct TypescriptModuleLoader;

impl ModuleLoader for TypescriptModuleLoader {
    fn resolve(&self, specifier: &str, referrer: &str, _is_main: bool) -> Result<ModuleSpecifier> {
        Ok(deno_core::resolve_import(specifier, referrer)?)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<ModuleSpecifier>,
        _is_dyn_import: bool,
    ) -> Pin<Box<ModuleSourceFuture>> {
        let module_specifier = module_specifier.clone();
        Box::pin(async move {
            let path = module_specifier
                .to_file_path()
                .map_err(|_| anyhow!("Only file: URLs are supported."))?;

            let media_type = MediaType::from(&path);
            let (module_type, should_transpile) = match MediaType::from(&path) {
                MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                    (ModuleType::JavaScript, false)
                }
                MediaType::Jsx => (ModuleType::JavaScript, true),
                MediaType::TypeScript
                | MediaType::Mts
                | MediaType::Cts
                | MediaType::Dts
                | MediaType::Dmts
                | MediaType::Dcts
                | MediaType::Tsx => (ModuleType::JavaScript, true),
                MediaType::Json => (ModuleType::Json, false),
                _ => panic!("Unknown extension {:?}", path.extension()),
            };

            let code = std::fs::read_to_string(&path)?;
            let code = if should_transpile {
                let parsed = deno_ast::parse_module(ParseParams {
                    specifier: module_specifier.to_string(),
                    source: SourceTextInfo::from_string(code),
                    media_type,
                    capture_tokens: false,
                    scope_analysis: false,
                    maybe_syntax: None,
                })?;
                parsed.transpile(&Default::default())?.text
            } else {
                code
            };
            let module = ModuleSource {
                code: code.into_bytes().into_boxed_slice(),
                module_type,
                module_url_specified: module_specifier.to_string(),
                module_url_found: module_specifier.to_string(),
            };
            Ok(module)
        })
    }
}
