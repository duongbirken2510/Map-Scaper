import * as esbuild from "esbuild";

await esbuild.build({
  entryPoints: ["index.mjs"],
  bundle: true,
  platform: "node",
  format: "cjs",
  banner: {
    js: "Object.defineProperty(globalThis,'__importMetaUrl',{get:()=>'file://'+__filename});",
  },
  define: {
    "import.meta.url": "globalThis.__importMetaUrl",
  },
  outfile: "bundle.cjs",
});
console.log("Built zalo-sidecar/bundle.cjs successfully.");
