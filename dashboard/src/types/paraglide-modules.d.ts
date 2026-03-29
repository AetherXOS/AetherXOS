declare module '$lib/paraglide/messages' {
	export const m: Record<string, (...args: unknown[]) => string>;
}

declare module '$lib/paraglide/runtime' {
	export function setLocale(locale: string, options?: { reload?: boolean }): Promise<void>;
	export function deLocalizeUrl(url: string | URL): URL;
}
