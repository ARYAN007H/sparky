import { promises as fs } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const english: Record<string, string> = {
  'channels.savedRestartRequired': 'Channel saved. Restart the app to activate it.',
  'channels.connect': 'Connect',
  'channels.discord.oauthStartFailed': "Couldn't start Discord sign-in. Try again.",
  'mcp.json.loading': 'Loading…',
  'settings.embeddings.testUnavailableCustom':
    'Testing a custom endpoint is not supported yet — save it and check the status on the Embeddings panel.',
  connectProvider: 'Connect',
  'walletSend.title': 'Send',
  'walletSend.to': 'To',
  'walletSend.availableBalance': 'Available: {{amount}} {{symbol}}',
  'settings.skillsRunner.error.listWorkflows': 'Failed to load workflows:',
  'conversations.runMode.plan': 'Plan',
  'channels.oauth.connectRecoverFailed':
    "Couldn't finish connecting {channel}. The sign-in window closed before access was granted — try again.",
};
const locales = ['zh-CN','hi','es','ar','fr','bn','pt','de','ru','id','it','ja','ko','pl','tr'];
const translateKeys = [
  'channels.discord.oauthStartFailed', 'mcp.json.loading',
  'settings.embeddings.testUnavailableCustom', 'walletSend.to',
  'walletSend.availableBalance', 'settings.skillsRunner.error.listWorkflows',
  'conversations.runMode.plan', 'channels.oauth.connectRecoverFailed',
] as const;
const placeholderPattern = /\{\{?[A-Za-z0-9_]+\}?\}/g;

async function translate(locale: string, values: string[]): Promise<string[]> {
  const tokens = new Map<string, string>();
  const safe = values.map((value, row) => value.replace(placeholderPattern, placeholder => {
    const token = `ZXQPH${row}_${tokens.size}QXZ`;
    tokens.set(token, placeholder);
    return token;
  }));
  const body = new URLSearchParams({client:'gtx',sl:'en',tl:locale,dt:'t',q:safe.join('\n')});
  const response = await fetch('https://translate.googleapis.com/translate_a/single', {
    method:'POST',headers:{'content-type':'application/x-www-form-urlencoded;charset=UTF-8'},body,
  });
  if (!response.ok) throw new Error(`${locale}: translation service returned ${response.status}`);
  const data = await response.json() as [string[][][]];
  const output = data[0].map(segment => segment[0]?.[0] ?? '').join('').split(/\r?\n/).map(value => value.trim());
  if (output.length !== values.length) throw new Error(`${locale}: expected ${values.length}, received ${output.length}`);
  return output.map(value => {
    for (const [token, placeholder] of tokens) value = value.replaceAll(token, placeholder);
    return value;
  });
}

async function append(file: string, entries: Record<string, string>) {
  let source = await fs.readFile(file, 'utf8');
  const missing = Object.keys(entries).filter(key => !new RegExp(`['\"]${key.replace(/[.*+?^${}()|[\\]\\]/g, '\\$&')}['\"]\\s*:`).test(source));
  if (missing.length !== Object.keys(entries).length) throw new Error(`${file}: some new keys already exist`);
  const lines = Object.entries(entries).map(([key, value]) => `  ${JSON.stringify(key)}: ${JSON.stringify(value)},`).join('\n');
  source = source.replace(/\n};\s*\n\nexport default /, `\n\n  // UI keys referenced by the app but missing from the locale maps.\n${lines}\n};\n\nexport default `);
  await fs.writeFile(file, source);
}

async function main() {
  const dir = path.join(process.cwd(), 'app/src/lib/i18n');
  await append(path.join(dir, 'en.ts'), english);
  for (const locale of locales) {
    const mod = await import(pathToFileURL(path.join(dir, `${locale}.ts`)).href);
    const existing = mod.default as Record<string, string>;
    const translated = await translate(locale, translateKeys.map(key => english[key]));
    const entries: Record<string, string> = Object.fromEntries(translateKeys.map((key,index) => [key,translated[index]]));
    entries['channels.savedRestartRequired'] = existing['channels.discord.savedRestartRequired'] ?? english['channels.savedRestartRequired'];
    entries['channels.connect'] = existing['channels.yuanbao.connect'] ?? english['channels.connect'];
    entries.connectProvider = existing['settings.ai.connectProvider'] ?? english.connectProvider;
    entries['walletSend.title'] = existing['walletBalances.send'] ?? english['walletSend.title'];
    await append(path.join(dir, `${locale}.ts`), entries);
    console.log(`${locale}: added ${Object.keys(entries).length} entries`);
  }
}

main().catch(error => { console.error(error); process.exitCode = 1; });
