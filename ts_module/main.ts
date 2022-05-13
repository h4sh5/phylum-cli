import { Test } from "./test.ts"

const xxx = new Test();
Deno.core.print("Result: " + xxx.test() + "\n");
