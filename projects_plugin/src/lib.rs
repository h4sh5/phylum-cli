wit_bindgen_rust::import!("../phylum.wit");
wit_bindgen_rust::export!("../plugin.wit");

struct Plugin;

impl plugin::Plugin for Plugin {
    fn entry_point() {
        println!("Custom Project Formatter:");
        for project in phylum::projects() {
            println!("{} ~~~ {}", project.id, project.name);
        }
    }
}
