#[no_mangle]
pub extern "C" fn entry_point() {
    unsafe {
        print_projects();
    }
}

#[link(wasm_import_module = "phylum")]
extern "C" {
    fn print_projects();
}
