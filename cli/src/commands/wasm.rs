use clap::ArgMatches;
use phylum_types::types::project::ProjectSummaryResponse;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::sync::WasiCtxBuilder;
use wasmtime_wasi::WasiCtx;

use crate::api::PhylumApi;
use crate::commands::wasm::phylum::{Phylum, Project};
use crate::commands::wasm::plugin::{Plugin, PluginData};
use crate::commands::{CommandResult, ExitCode};

wit_bindgen_wasmtime::export!({ paths: ["../phylum.wit"], async: * });
wit_bindgen_wasmtime::import!({ paths: ["../plugin.wit"], async: * });

pub async fn handle_wasm(api: PhylumApi, matches: &ArgMatches) -> CommandResult {
    let wasm = matches.value_of("FILE").unwrap();

    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, wasm)?;

    // Define functions to be called from WASM.
    let mut linker: Linker<WasmContext> = Linker::new(&engine);
    phylum::add_to_linker(&mut linker, |data| data)?;

    wasmtime_wasi::add_to_linker(&mut linker, |data| &mut data.wasi)?;

    let context = WasmContext::new(api)?;
    let mut store = Store::new(&engine, context);

    let instance = linker.instantiate(&mut store, &module)?;

    // Call function from WASM plugin.
    let plugin = Plugin::new(&mut store, &instance, |data| &mut data.plugin)?;
    plugin.entry_point(&mut store).await?;

    Ok(ExitCode::Ok.into())
}

pub struct WasmContext {
    plugin: PluginData,
    phylum: PhylumApi,
    wasi: WasiCtx,
}

impl WasmContext {
    fn new(phylum: PhylumApi) -> anyhow::Result<Self> {
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_args()?
            .build();

        Ok(Self {
            plugin: PluginData::default(),
            phylum,
            wasi,
        })
    }
}

#[wit_bindgen_wasmtime::async_trait]
impl Phylum for WasmContext {
    async fn projects(&mut self) -> Vec<Project> {
        let mut projects = self.phylum.get_projects().await.unwrap_or_default();
        projects.drain(..).map(Project::from).collect()
    }
}

impl From<ProjectSummaryResponse> for Project {
    fn from(response: ProjectSummaryResponse) -> Self {
        Self {
            name: response.name,
            id: response.id,
        }
    }
}
