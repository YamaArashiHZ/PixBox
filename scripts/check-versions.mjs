import { readFile } from "node:fs/promises";

const semverPattern =
  /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

function cargoPackageVersion(content, packageName) {
  const blocks = content.split(/(?=\[\[package\]\])/);
  const block = blocks.find((candidate) => {
    const name = candidate.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
    return name === packageName;
  });
  return block?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
}

function cargoManifestVersion(content) {
  const packageSection = content.match(/\[package\]([\s\S]*?)(?=\n\[|$)/)?.[1];
  return packageSection?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
}

const [packageJson, packageLock, tauriConfig, cargoToml, cargoLock] =
  await Promise.all([
    readJson("package.json"),
    readJson("package-lock.json"),
    readJson("src-tauri/tauri.conf.json"),
    readFile("src-tauri/Cargo.toml", "utf8"),
    readFile("src-tauri/Cargo.lock", "utf8"),
  ]);

const versions = new Map([
  ["package.json", packageJson.version],
  ["package-lock.json", packageLock.version],
  ["package-lock.json root package", packageLock.packages?.[""]?.version],
  ["src-tauri/tauri.conf.json", tauriConfig.version],
  ["src-tauri/Cargo.toml", cargoManifestVersion(cargoToml)],
  ["src-tauri/Cargo.lock", cargoPackageVersion(cargoLock, packageJson.name)],
]);

const expected = packageJson.version;
const errors = [];

if (!semverPattern.test(expected)) {
  errors.push(`package.json has an invalid SemVer version: ${expected}`);
}

for (const [file, version] of versions) {
  if (!version) {
    errors.push(`${file} does not contain the PixBox version`);
  } else if (version !== expected) {
    errors.push(`${file} has version ${version}; expected ${expected}`);
  }
}

const tagIndex = process.argv.indexOf("--tag");
if (tagIndex !== -1) {
  const tag = process.argv[tagIndex + 1];
  if (!tag) {
    errors.push("--tag requires a value");
  } else if (tag !== `v${expected}`) {
    errors.push(`release tag ${tag} does not match version v${expected}`);
  }
}

if (process.argv.includes("--release")) {
  const updater = tauriConfig.plugins?.updater;
  if (typeof updater?.pubkey !== "string" || updater.pubkey.trim() === "") {
    errors.push("src-tauri/tauri.conf.json is missing plugins.updater.pubkey");
  }
  if (
    !Array.isArray(updater?.endpoints) ||
    !updater.endpoints.includes(
      "https://github.com/YamaArashiHZ/PixBox/releases/latest/download/latest.json",
    )
  ) {
    errors.push("the production GitHub updater endpoint is not configured");
  }
  if (tauriConfig.bundle?.createUpdaterArtifacts !== true) {
    errors.push("bundle.createUpdaterArtifacts must be true for a release");
  }
}

if (errors.length > 0) {
  console.error(errors.map((error) => `- ${error}`).join("\n"));
  process.exitCode = 1;
} else {
  console.log(`All PixBox versions match ${expected}.`);
}
