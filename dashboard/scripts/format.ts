import { spawnSync } from 'node:child_process';

const prettierTargets = [
	'src/**/*.{svelte,ts,js,json,css}',
	'messages/*.json',
	'scripts/**/*.{ts,js}',
	'package.json',
	'.prettierrc',
	'vite.config.ts'
];

const result = spawnSync('npx', ['prettier', '--write', ...prettierTargets], {
	stdio: 'inherit',
	shell: process.platform === 'win32'
});

if (typeof result.status === 'number' && result.status !== 0) {
	process.exit(result.status);
}
