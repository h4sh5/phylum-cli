#[no_mangle]
extern "C" fn main() -> i32 {
    return unsafe { simple() };
}

#[link(wasm_import_module = "phylum")]
extern "C" {
    fn simple() -> i32;
}
