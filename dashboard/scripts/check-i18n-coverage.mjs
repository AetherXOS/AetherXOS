import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const enPath = path.join(root, 'messages', 'en.json');
const trPath = path.join(root, 'messages', 'tr.json');

const en = JSON.parse(fs.readFileSync(enPath, 'utf8'));
const tr = JSON.parse(fs.readFileSync(trPath, 'utf8'));

const enKeys = new Set(Object.keys(en));
const trKeys = new Set(Object.keys(tr));

const missingInTr = [...enKeys].filter((key) => !trKeys.has(key));
const missingInEn = [...trKeys].filter((key) => !enKeys.has(key));

if (missingInTr.length === 0 && missingInEn.length === 0) {
	console.log('[i18n] coverage check passed');
	process.exit(0);
}

console.error('[i18n] coverage check failed');
if (missingInTr.length > 0) {
	console.error(`Missing in tr.json (${missingInTr.length}):`);
	for (const key of missingInTr) console.error(` - ${key}`);
}
if (missingInEn.length > 0) {
	console.error(`Missing in en.json (${missingInEn.length}):`);
	for (const key of missingInEn) console.error(` - ${key}`);
}

process.exit(1);
