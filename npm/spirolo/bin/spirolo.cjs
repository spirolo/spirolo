#!/usr/bin/env node
'use strict';
const { spawnSync } = require('node:child_process');
const { resolveBinary } = require('../resolve.cjs');
try {
  const result = spawnSync(resolveBinary(), process.argv.slice(2), { stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.signal) process.kill(process.pid, result.signal);
  else process.exitCode = result.status ?? 2;
} catch (error) {
  console.error(error.message);
  process.exitCode = 2;
}
