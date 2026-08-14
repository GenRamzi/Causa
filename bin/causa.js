#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const args = process.argv.slice(2);
const command = args[0] || 'help';
const root = path.resolve(__dirname, '..');

function runRust(rest) {
  const result = spawnSync('cargo', ['run', '--quiet', '-p', 'causa', '--', ...rest], { cwd: root, stdio: 'inherit' });
  if (result.error) return false;
  process.exitCode = result.status || 0;
  return true;
}

if (command !== 'demo' && fs.existsSync(path.join(root, 'Cargo.toml')) && process.env.CAUSA_USE_RUST !== '0') {
  if (runRust(args)) process.exit();
}

if (command === 'demo') {
  console.log('Causa demo · zero install, zero API key');
  console.log('This package includes a deterministic failure fixture.');
  console.log('Run from a source checkout for the full Rust workflow:');
  console.log('  cargo run -p causa -- demo');
  console.log(`Viewer: file://${path.join(root, 'viewer', 'index.html')}`);
  process.exit(0);
}

console.log('Causa — the black box for AI agents');
console.log('Install Rust and run `cargo run -p causa -- --help` for the complete CLI.');
