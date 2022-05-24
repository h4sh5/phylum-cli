import { Test } from "./test.ts"

async function projects() {
    const projects = await Deno.core.opAsync("projects");
    for (let i = 0; i < projects.length; i++) {
        Deno.core.print("Project: " + projects[i].name + "\n");
    }
}

projects();

// const xxx = new Test();
// Deno.core.print("Result: " + xxx.test() + "\n");
