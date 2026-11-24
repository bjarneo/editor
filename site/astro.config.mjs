import { defineConfig } from "astro/config";

// GitHub Pages serves the site at https://bjarneo.github.io/editor/.
export default defineConfig({
  site: "https://bjarneo.github.io",
  base: "/editor",
  output: "static",
  build: { inlineStylesheets: "always" },
});
