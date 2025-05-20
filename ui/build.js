import * as esbuild from "esbuild";
import * as fs from "fs";
import mustache from "mustache";
import * as path from "path";
import * as process from "node:process";
import * as child_process from "node:child_process";
import sveltePlugin from "esbuild-svelte";
import { sveltePreprocess } from "svelte-preprocess";

fs.mkdirSync("build", { recursive: true });

console.log("Build the extension");
await fs.promises.rm(path.join("extension", "build"), {
    force: true,
    recursive: true,
});
child_process.spawnSync(
    "cargo",
    ["build", "--target", "wasm32-unknown-unknown", "--release"],
    {
        cwd: "extension",
        stdio: "inherit",
    },
);
child_process.spawnSync(
    "wasm-bindgen",
    [
        `target/wasm32-unknown-unknown/release/extension.wasm`,
        "--out-dir",
        "build",
        "--target",
        "web",
    ],
    {
        cwd: "extension",
        stdio: "inherit",
    },
);

const wasmPlugin = {
    name: "wasm",
    setup(build) {
        build.onResolve({ filter: /\.wasm$/ }, args => ({
            path: path.isAbsolute(args.path)
                ? args.path
                : path.join(args.resolveDir, args.path),
            namespace: "wasm",
        }));
        build.onLoad({ filter: /.*/, namespace: "wasm" }, async args => ({
            contents: await fs.promises.readFile(args.path),
            loader: "binary",
        }));
    },
};

function withExtension(pathString, ext) {
    const parsedPath = path.parse(pathString);
    return path.format({
        root: parsedPath.root,
        dir: parsedPath.dir,
        name: parsedPath.name,
        ext,
    });
}

console.log("Initialize esbuild");
const context = await esbuild.context({
    entryPoints: [
        { in: path.join("src", "main.ts"), out: "main" },
        { in: path.join("src", "styles.css"), out: "styles" },
    ],
    mainFields: ["svelte", "browser", "module", "main"],
    conditions: ["svelte", "browser"],
    outdir: "build",
    bundle: true,
    loader: { ".ttf": "dataurl" },
    target: ["es2022"],
    format: "esm",
    write: false,
    minify: process.env.MODE === "production",
    plugins: [
        sveltePlugin({
            compilerOptions: { css: "injected", runes: true },
            preprocess: sveltePreprocess(),
        }),
        {
            name: "webworker",
            setup(build) {
                build.onResolve({ filter: /\.wts$/ }, args => {
                    const resolvedPath = path.isAbsolute(args.path)
                        ? args.path
                        : path.join(args.resolveDir, args.path);
                    return {
                        path: withExtension(resolvedPath, ".js"),
                        namespace: "webworker",
                        watchFiles: [withExtension(resolvedPath, ".ts")],
                    };
                });
                build.onLoad(
                    { filter: /.*/, namespace: "webworker" },
                    async args => {
                        const parsedPath = path.parse(args.path);
                        let contents = null;
                        await esbuild.build({
                            entryPoints: [
                                {
                                    in: withExtension(args.path, ".ts"),
                                    out: parsedPath.name,
                                },
                            ],
                            mainFields: ["browser", "module", "main"],
                            outdir: "build",
                            bundle: true,
                            target: ["es2022"],
                            format: "esm",
                            write: false,
                            minify: process.env.MODE === "production",
                            plugins: [
                                wasmPlugin,
                                {
                                    name: "bundle",
                                    setup(build) {
                                        build.onEnd(result => {
                                            if (result.errors.length === 0) {
                                                contents =
                                                    result.outputFiles[0]
                                                        .contents;
                                            }
                                        });
                                    },
                                },
                            ],
                        });
                        return {
                            contents: contents,
                            loader: "dataurl",
                        };
                    },
                );
            },
        },
        wasmPlugin,
        {
            name: "bundle",
            setup(build) {
                build.onEnd(result => {
                    if (result.errors.length === 0) {
                        fs.writeFileSync(
                            path.join("build", "index.html"),
                            mustache.render(
                                fs
                                    .readFileSync(
                                        path.join("src", "index.mustache"),
                                    )
                                    .toString(),
                                {
                                    title: "Kairos",
                                    script: result.outputFiles[0].text,
                                    styles: result.outputFiles[1].text,
                                },
                            ),
                        );
                        if (process.argv.includes("--watch")) {
                            console.log(
                                `\x1b[32m✓\x1b[0m ${new Date().toLocaleString()}`,
                            );
                        }
                    }
                });
            },
        },
    ],
});

if (process.argv.includes("--watch")) {
    await context.watch();
} else {
    await context.rebuild();
    await context.dispose();
}
