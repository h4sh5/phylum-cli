use clap::ArgMatches;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::sync::WasiCtxBuilder;
use wasmtime_wasi::WasiCtx;

use crate::api::PhylumApi;
use crate::commands::{CommandResult, ExitCode};

// TODO: Async not supported yet.
// wit_bindgen_wasmtime::export!("../phylum.wit");

pub async fn handle_wasm(api: PhylumApi, matches: &ArgMatches) -> CommandResult {
    let wasm = matches.value_of("FILE").unwrap();

    let mut config = Config::new();
    config.async_support(true);
    // config.wasm_multi_memory(true); // TODO: Required?
    // config.wasm_memory64(true); // TODO: Required?
    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, wasm)?;

    // Define functions to be called from WASM.
    let mut linker: Linker<WasmContext> = Linker::new(&engine);
    phylum::add_to_linker(&mut linker)?;

    wasmtime_wasi::add_to_linker(&mut linker, |data| &mut data.wasi)?;

    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .build();
    let context = WasmContext { phylum: api, wasi };
    let mut store = Store::new(&engine, context);

    let instance = linker.instantiate(&mut store, &module)?;

    // Call function from WASM plugin.
    let entry_point = instance
        .get_func(&mut store, "entry-point")
        .expect("`entry-point` was not an exported function");
    entry_point.call_async(&mut store, &[], &mut []).await?;

    Ok(ExitCode::Ok.into())
}

pub struct WasmContext {
    phylum: PhylumApi,
    wasi: WasiCtx,
}

impl WasmContext {
    async fn projects(&mut self) -> Vec<phylum::Project> {
        let mut projects = self.phylum.get_projects().await.unwrap_or_default();
        projects.drain(..).map(phylum::Project::from).collect()
    }
}

// TODO: Since wit-bindgen uses traits, async isn't supported. This is a slight modification to the
// auto-generated non-async code to not use traits and enable async usage.
pub mod phylum {
    #[allow(unused_imports)]
    use wit_bindgen_wasmtime::{anyhow, wasmtime};
    #[derive(Clone)]
    pub struct Project {
        pub name: String,
        pub id: String,
    }
    impl std::fmt::Debug for Project {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Project")
                .field("name", &self.name)
                .field("id", &self.id)
                .finish()
        }
    }

    use phylum_types::types::project::ProjectSummaryResponse;
    impl From<ProjectSummaryResponse> for Project {
        fn from(response: ProjectSummaryResponse) -> Self {
            Self {
                name: response.name,
                id: response.id,
            }
        }
    }

    use crate::commands::wasm::WasmContext;
    pub fn add_to_linker(linker: &mut wasmtime::Linker<WasmContext>) -> anyhow::Result<()> {
        use wit_bindgen_wasmtime::rt::get_func;
        use wit_bindgen_wasmtime::rt::get_memory;
        linker.func_wrap1_async(
            "phylum",
            "projects",
            move |mut caller: wasmtime::Caller<'_, WasmContext>, arg0: i32| {
                Box::new(async move {
                    let func = get_func(&mut caller, "canonical_abi_realloc")?;
                    let func_canonical_abi_realloc =
                        func.typed::<(i32, i32, i32, i32), i32, _>(&caller)?;
                    let memory = &get_memory(&mut caller, "memory")?;
                    let result = caller.data_mut().projects().await;
                    let vec3 = result;
                    let len3 = vec3.len() as i32;
                    let result3 = func_canonical_abi_realloc
                        .call_async(&mut caller, (0, 0, 4, len3 * 16))
                        .await?;
                    for (i, e) in vec3.into_iter().enumerate() {
                        let base = result3 + (i as i32) * 16;
                        {
                            let Project {
                                name: name0,
                                id: id0,
                            } = e;
                            let vec1 = name0;
                            let ptr1 = func_canonical_abi_realloc
                                .call_async(&mut caller, (0, 0, 1, vec1.len() as i32))
                                .await?;
                            let caller_memory = memory.data_mut(&mut caller);
                            caller_memory.store_many(ptr1, vec1.as_bytes())?;
                            caller_memory.store(
                                base + 4,
                                wit_bindgen_wasmtime::rt::as_i32(vec1.len() as i32),
                            )?;
                            caller_memory
                                .store(base + 0, wit_bindgen_wasmtime::rt::as_i32(ptr1))?;
                            let vec2 = id0;
                            let ptr2 = func_canonical_abi_realloc
                                .call_async(&mut caller, (0, 0, 1, vec2.len() as i32))
                                .await?;
                            let caller_memory = memory.data_mut(&mut caller);
                            caller_memory.store_many(ptr2, vec2.as_bytes())?;
                            caller_memory.store(
                                base + 12,
                                wit_bindgen_wasmtime::rt::as_i32(vec2.len() as i32),
                            )?;
                            caller_memory
                                .store(base + 8, wit_bindgen_wasmtime::rt::as_i32(ptr2))?;
                        }
                    }
                    let caller_memory = memory.data_mut(&mut caller);
                    caller_memory.store(arg0 + 4, wit_bindgen_wasmtime::rt::as_i32(len3))?;
                    caller_memory.store(arg0 + 0, wit_bindgen_wasmtime::rt::as_i32(result3))?;
                    Ok(())
                })
            },
        )?;
        Ok(())
    }
    use wit_bindgen_wasmtime::rt::RawMem;
}
