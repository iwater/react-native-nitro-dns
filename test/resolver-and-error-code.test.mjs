import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import { execFileSync } from 'node:child_process';
import Module, { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = path.resolve(__dirname, '..');
const require = createRequire(import.meta.url);

const tmpOut = fs.mkdtempSync(path.join(os.tmpdir(), 'nitro-dns-test-'));
execFileSync(
  path.join(PKG_ROOT, 'node_modules/.bin/tsc'),
  [
    '--module', 'commonjs',
    '--moduleResolution', 'node',
    '--target', 'ES2020',
    '--esModuleInterop',
    '--skipLibCheck',
    '--outDir', tmpOut,
    path.join(PKG_ROOT, 'src/index.ts'),
  ],
  { stdio: 'pipe' }
);

let rejectWithMessage = 'no record found for Query { name: Name("127.0.0.1."), query_type: A, query_class: IN }';

const mockResolver = {
  resolveipv4: async () => { throw new Error(rejectWithMessage); },
  resolveipv6: async () => { throw new Error(rejectWithMessage); },
  resolveAny: async () => { throw new Error(rejectWithMessage); },
  getServers: () => JSON.stringify(['8.8.8.8']),
  setServers: () => {},
  cancel: () => {},
  clearCache: () => {},
  setLocalAddress: () => {},
};

const mockNitroDns = {
  createResolver: () => mockResolver,
  resolve4: async () => { throw new Error(rejectWithMessage); },
  resolve6: async () => { throw new Error(rejectWithMessage); },
  resolveAny: async () => { throw new Error(rejectWithMessage); },
  getServers: () => JSON.stringify(['8.8.8.8']),
  lookup: (host) => {
    if (host === '127.0.0.1' || host === '::1') {
      throw new Error(`Native lookup must not be called for IP literal: ${host}`);
    }
    return JSON.stringify(['93.184.216.34']);
  },
};

const origLoad = Module._load;
Module._load = function (request, parent, isMain) {
  if (request === 'react-native-nitro-modules') {
    return {
      NitroModules: {
        createHybridObject: () => mockNitroDns,
      },
    };
  }
  return origLoad.apply(this, arguments);
};

const dns = require(path.join(tmpOut, 'index.js'));

test.after(() => {
  fs.rmSync(tmpOut, { recursive: true, force: true });
});

test('promises.Resolver supports standalone construction without arguments or with options', async () => {
  const r1 = new dns.promises.Resolver();
  const r2 = new dns.promises.Resolver({ timeout: 2000, tries: 2 });
  assert.deepEqual(r1.getServers(), ['8.8.8.8']);
  assert.deepEqual(r2.getServers(), ['8.8.8.8']);
});

test('DNS errors without err.code are normalized to Node standard codes (ENODATA / ENOTFOUND / ETIMEOUT)', async () => {
  const resolver = new dns.promises.Resolver();

  rejectWithMessage = 'no record found for Query { name: Name("127.0.0.1."), query_type: A, query_class: IN }';
  await assert.rejects(
    () => resolver.resolve4('127.0.0.1'),
    (err) => err instanceof Error && err.code === 'ENODATA'
  );
  await assert.rejects(
    () => dns.promises.resolve4('127.0.0.1'),
    (err) => err instanceof Error && err.code === 'ENODATA'
  );
  await new Promise((resolve) => {
    dns.resolve4('127.0.0.1', (err) => {
      assert.ok(err && err.code === 'ENODATA');
      resolve();
    });
  });

  rejectWithMessage = 'NXDOMAIN: Non-Existent Domain';
  await assert.rejects(
    () => resolver.resolve6('missing.invalid'),
    (err) => err instanceof Error && err.code === 'ENOTFOUND'
  );

  rejectWithMessage = 'request timed out';
  await assert.rejects(
    () => dns.promises.resolveAny('slow.invalid'),
    (err) => err instanceof Error && err.code === 'ETIMEOUT'
  );
});

test('dns.lookup and dns.promises.lookup short-circuit IPv4/IPv6 literals including { all: true }', async () => {
  await new Promise((resolve) => {
    dns.lookup('127.0.0.1', (err, addr, fam) => {
      assert.equal(err, null);
      assert.equal(addr, '127.0.0.1');
      assert.equal(fam, 4);
      resolve();
    });
  });

  await new Promise((resolve) => {
    dns.lookup('127.0.0.1', { all: true }, (err, addrs) => {
      assert.equal(err, null);
      assert.deepEqual(addrs, [{ address: '127.0.0.1', family: 4 }]);
      resolve();
    });
  });

  await new Promise((resolve) => {
    dns.lookup('::1', { all: true }, (err, addrs) => {
      assert.equal(err, null);
      assert.deepEqual(addrs, [{ address: '::1', family: 6 }]);
      resolve();
    });
  });

  const pSingle = await dns.promises.lookup('127.0.0.1');
  assert.deepEqual(pSingle, { address: '127.0.0.1', family: 4 });

  const pAll = await dns.promises.lookup('127.0.0.1', { all: true });
  assert.deepEqual(pAll, [{ address: '127.0.0.1', family: 4 }]);
});
