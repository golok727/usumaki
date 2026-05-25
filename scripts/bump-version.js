import { readFile, writeFile, appendFile } from 'node:fs/promises';
import path from 'node:path';
import semver from 'semver';

const CLI_MANIFEST = 'crates/cli/Cargo.toml';

const CARGO_MANIFESTS = ['crates/cli/Cargo.toml', 'crates/uzumaki/Cargo.toml'];
const PACKAGE_JSONS = ['crates/uzumaki/package.json'];

const RELEASE_TYPES = [
  'major',
  'minor',
  'patch',
  'prerelease',
  'premajor',
  'preminor',
  'prepatch',
];

function usage() {
  console.error(
    'Usage: node scripts/bump-version.js <major|minor|patch|prerelease|x.y.z> [--preid <id>]',
  );
  process.exit(1);
}

function parseArgs(argv) {
  let target = null;
  let preid = 'alpha';
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--preid') {
      preid = argv[++i];
      if (!preid) usage();
    } else if (!target) {
      target = arg;
    } else {
      usage();
    }
  }
  if (!target) usage();
  return { target, preid };
}

function readPackageSectionVersion(toml) {
  const match = toml.match(/^\[package\]\s*$([\s\S]*?)(?=^\[|\Z)/m);
  if (!match) return null;
  const versionMatch = match[1].match(/^\s*version\s*=\s*"([^"]+)"/m);
  return versionMatch ? versionMatch[1] : null;
}

function replacePackageSectionVersion(toml, next) {
  return toml.replace(
    /^(\[package\]\s*$[\s\S]*?^\s*version\s*=\s*")[^"]+(")/m,
    `$1${next}$2`,
  );
}

async function readCurrentVersion(root) {
  const toml = await readFile(path.join(root, CLI_MANIFEST), 'utf8');
  const current = readPackageSectionVersion(toml);
  if (!current) {
    console.error(`Could not find a [package] version in ${CLI_MANIFEST}`);
    process.exit(1);
  }
  return current;
}

function computeNext(current, target, preid) {
  if (RELEASE_TYPES.includes(target)) {
    const next = semver.inc(current, target, preid);
    if (!next) {
      console.error(`Failed to bump ${current} with release type "${target}"`);
      process.exit(1);
    }
    return next;
  }
  const explicit = semver.valid(target);
  if (!explicit) {
    console.error(
      `"${target}" is neither a release type nor a valid semver version`,
    );
    usage();
  }
  if (!semver.gt(explicit, current)) {
    console.error(
      `Explicit version ${explicit} is not greater than current ${current}`,
    );
    process.exit(1);
  }
  return explicit;
}

async function updateCargo(root, file, next) {
  const full = path.join(root, file);
  const toml = await readFile(full, 'utf8');
  const updated = replacePackageSectionVersion(toml, next);
  if (readPackageSectionVersion(updated) !== next) {
    console.error(`Could not set [package] version to ${next} in ${file}`);
    process.exit(1);
  }
  await writeFile(full, updated);
}

async function updatePackageJson(root, file, next) {
  const full = path.join(root, file);
  const raw = await readFile(full, 'utf8');
  const pkg = JSON.parse(raw);
  pkg.version = next;
  const trailing = raw.endsWith('\n') ? '\n' : '';
  await writeFile(full, JSON.stringify(pkg, null, 2) + trailing);
}

async function main() {
  const { target, preid } = parseArgs(process.argv.slice(2));
  const root = process.cwd();

  const current = await readCurrentVersion(root);
  const next = computeNext(current, target, preid);

  for (const file of CARGO_MANIFESTS) await updateCargo(root, file, next);
  for (const file of PACKAGE_JSONS) await updatePackageJson(root, file, next);

  console.log(`${current} -> ${next}`);

  if (process.env.GITHUB_OUTPUT) {
    await appendFile(
      process.env.GITHUB_OUTPUT,
      `previous=${current}\nversion=${next}\n`,
    );
  }
}

main();
