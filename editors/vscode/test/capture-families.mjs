// Real endpoint capture for the family and Health views (#314): the `spirolo/rules` summary, each
// family's `{family}` answer, each rule's `{rule}` answer, and `spirolo/health`.
// node capture-families.mjs <binary> <absolute workspace> > families.json
import { LspClient } from '../../../corpus/lsp/client.mjs';
const [binary, root] = process.argv.slice(2);
const client = await LspClient.start({ binary, root });
const timeout = setTimeout(() => { client.child.kill(); process.exit(1); }, 120_000);
const result = async (method, params) => {
  const answer = await client.request(method, params);
  if (answer.error) throw new Error(`${method} ${JSON.stringify(params)}: ${answer.error.message}`);
  return answer.result;
};
try {
  await client.initialize();
  const pass = await client.waitChecked();
  const summary = await result('spirolo/rules', null);
  const family = {};
  for (const name of new Set(summary.rules.map((rule) => rule.family))) family[name] = await result('spirolo/rules', { family: name });
  const rule = {};
  for (const { rule: name } of summary.rules) rule[name] = await result('spirolo/rules', { rule: name });
  const health = await result('spirolo/health', null);
  console.log(JSON.stringify({
    _capture: 'Real spirolo lsp exchange on test/workspace; capture-families.mjs, #314',
    checked: pass.checked, summary, family, rule, health,
  }, null, 2));
} finally {
  await client.shutdown();
  clearTimeout(timeout);
}
