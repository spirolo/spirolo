// How the view.test.mjs fixtures were taken: drive `spirolo lsp` over stdio on a project, wait for
// the first `spirolo/checked`, then record `spirolo/tree` and `spirolo/file` for each given path.
// Usage: node capture.mjs <spirolo binary> <absolute project root> [repo-relative path...] > out.json
import { spawn } from 'node:child_process';
const [bin, root] = process.argv.slice(2);
const child = spawn(bin, ['lsp'], { cwd: root });
let buf = Buffer.alloc(0), next = 1; const waiters = new Map(); const out = {};
let onChecked;
child.stdout.on('data', (d) => {
  buf = Buffer.concat([buf, d]);
  for (;;) {
    const h = buf.indexOf('\r\n\r\n'); if (h < 0) return;
    const len = Number(/Content-Length: (\d+)/i.exec(buf.subarray(0, h).toString())[1]);
    if (buf.length < h + 4 + len) return;
    const msg = JSON.parse(buf.subarray(h + 4, h + 4 + len).toString());
    buf = buf.subarray(h + 4 + len);
    if (msg.id !== undefined && waiters.has(msg.id)) { waiters.get(msg.id)(msg); waiters.delete(msg.id); }
    else if (msg.method === 'spirolo/checked') { out.checked = msg.params; onChecked?.(); }
  }
});
const send = (m) => { const s = JSON.stringify({ jsonrpc: '2.0', ...m }); child.stdin.write(`Content-Length: ${Buffer.byteLength(s)}\r\n\r\n${s}`); };
const ask = (method, params) => new Promise((res) => { const id = next++; waiters.set(id, res); send({ id, method, params }); });
const uri = `file://${root}`;
await ask('initialize', { processId: null, rootUri: uri, capabilities: {}, workspaceFolders: [{ uri, name: 'fixture' }] });
const checked = new Promise((r) => (onChecked = r));
send({ method: 'initialized', params: {} });
await checked;
out.tree = (await ask('spirolo/tree', null)).result;
out.file = {};
for (const p of process.argv.slice(4)) out.file[p] = (await ask('spirolo/file', { uri: `${uri}/${p}` })).result;
await ask('shutdown', null); send({ method: 'exit' });
console.log(JSON.stringify(out, null, 2));
