<script lang="ts">
	import type { LocaleCode, ThemeMode } from '$lib/types';
	import { m } from '$lib/paraglide/messages';
	import { Languages, Palette } from 'lucide-svelte';

	interface Props {
		localTheme: ThemeMode;
		localLang: LocaleCode;
		onThemeChange: (v: ThemeMode) => void;
		onLangChange: (v: LocaleCode) => void;
	}

	let { localTheme, localLang, onThemeChange, onLangChange }: Props = $props();
</script>

<section id="interface" class="card rounded-2xl border border-white/5 bg-base-200/80 p-5 sm:p-6 space-y-5 scroll-mt-28">
	<div class="flex items-center gap-3 border-b border-white/5 pb-4">
		<div class="bg-info/15 flex h-9 w-9 items-center justify-center rounded-xl">
			<Palette size={18} class="text-info" />
		</div>
		<div>
			<div class="text-sm font-black uppercase tracking-wider">Interface</div>
			<div class="text-xs opacity-50">Theme and language preferences</div>
		</div>
	</div>

	<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
		<div id="ui-theme" class="form-control gap-2 scroll-mt-28">
			<label class="flex items-center gap-2 text-xs font-bold uppercase opacity-60">
				<Palette size={13} />{m.settings_theme()}
			</label>
		<select class="select bg-base-100" value={localTheme} onchange={(e) => onThemeChange((e.currentTarget as HTMLSelectElement).value as ThemeMode)}>
				<option value="dark">{m.settings_theme_dark()}</option>
				<option value="light">{m.settings_theme_light()}</option>
			</select>
		</div>
		<div id="ui-language" class="form-control gap-2 scroll-mt-28">
			<label class="flex items-center gap-2 text-xs font-bold uppercase opacity-60">
				<Languages size={13} />{m.settings_language()}
			</label>
		<select class="select bg-base-100" value={localLang} onchange={(e) => onLangChange((e.currentTarget as HTMLSelectElement).value as LocaleCode)}>
				<option value="en">{m.settings_language_en()}</option>
				<option value="tr">{m.settings_language_tr()}</option>
			</select>
		</div>
	</div>
</section>
