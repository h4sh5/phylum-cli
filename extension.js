async function main() {
    const projects = await Deno.core.opAsync("projects");
    for (let i = 0; i < projects.length; i++) {
        Deno.core.print("Project: " + projects[i] + "\n");
    }
}
main();
