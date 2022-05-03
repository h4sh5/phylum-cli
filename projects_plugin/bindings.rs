pub mod phylum {
  #[allow(unused_imports)]
  use wit_bindgen_wasmtime::{wasmtime, anyhow};
  #[derive(Clone)]
  pub struct Project {
    pub name: String,
    pub id: String,
  }
  impl std::fmt::Debug for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.debug_struct("Project").field("name", &self.name).field("id", &self.id).finish()}
  }
  pub trait Phylum: Sized {
    fn projects(&mut self,) -> Vec<Project>;
    
  }
  
  pub fn add_to_linker<T, U>(linker: &mut wasmtime::Linker<T>, get: impl Fn(&mut T) -> &mut U+ Send + Sync + Copy + 'static) -> anyhow::Result<()> 
  where U: Phylum
  {
    use wit_bindgen_wasmtime::rt::get_memory;
    use wit_bindgen_wasmtime::rt::get_func;
    linker.func_wrap("phylum", "projects", move |mut caller: wasmtime::Caller<'_, T>,arg0:i32| {
      
      let func = get_func(&mut caller, "canonical_abi_realloc")?;
      let func_canonical_abi_realloc = func.typed::<(i32, i32, i32, i32), i32, _>(&caller)?;
      let memory = &get_memory(&mut caller, "memory")?;
      let host = get(caller.data_mut());
      let result = host.projects();
      let vec3 = result;
      let len3 = vec3.len() as i32;
      let result3 = func_canonical_abi_realloc.call(&mut caller, (0, 0, 4, len3 * 16))?;
      for (i, e) in vec3.into_iter().enumerate() {
        let base = result3 + (i as i32) * 16;
        {
          let Project{ name:name0, id:id0, } = e;
          let vec1 = name0;
          let ptr1 = func_canonical_abi_realloc.call(&mut caller, (0, 0, 1, vec1.len() as i32))?;
          let caller_memory = memory.data_mut(&mut caller);
          caller_memory.store_many(ptr1, vec1.as_bytes())?;
          caller_memory.store(base + 4, wit_bindgen_wasmtime::rt::as_i32(vec1.len() as i32))?;
          caller_memory.store(base + 0, wit_bindgen_wasmtime::rt::as_i32(ptr1))?;
          let vec2 = id0;
          let ptr2 = func_canonical_abi_realloc.call(&mut caller, (0, 0, 1, vec2.len() as i32))?;
          let caller_memory = memory.data_mut(&mut caller);
          caller_memory.store_many(ptr2, vec2.as_bytes())?;
          caller_memory.store(base + 12, wit_bindgen_wasmtime::rt::as_i32(vec2.len() as i32))?;
          caller_memory.store(base + 8, wit_bindgen_wasmtime::rt::as_i32(ptr2))?;
        }}let caller_memory = memory.data_mut(&mut caller);
        caller_memory.store(arg0 + 4, wit_bindgen_wasmtime::rt::as_i32(len3))?;
        caller_memory.store(arg0 + 0, wit_bindgen_wasmtime::rt::as_i32(result3))?;
        Ok(())
      })?;
      Ok(())
    }
    use wit_bindgen_wasmtime::rt::RawMem;
  }
  