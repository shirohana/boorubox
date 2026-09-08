// shadcn's `IsMobile` hook, which the sidebar copy-in reads to switch to its
// phone layout (a sheet) under 768px. Overridden to never match: BooruBox is a
// desktop app, its window has a minimum width (tauri.conf.json), and under the
// whole-app zoom the CSS viewport can still drop below the breakpoint — where
// the sheet looped open/close at a 770px window (2026-09-10). A phone layout
// has no place in it. `pnpm dlx shadcn-svelte add sidebar` overwrites this
// file: re-apply this.
import { MediaQuery } from "svelte/reactivity";

export class IsMobile extends MediaQuery {
	constructor() {
		// A query no window satisfies.
		super("max-width: 0px");
	}
}
