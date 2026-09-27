/**
 * Whether `pathname` is `href` or a page under it (`settings-pages` design
 * D5): shared by the app sidebar (`Settings` lit on every `/settings/*`) and
 * the settings nav, so "current" is decided once. The boundary slash keeps
 * `/settings` from claiming a sibling `/settingsx`, and `/` never claims
 * another screen because nothing starts with `//`.
 */
export function isCurrentPath(pathname: string, href: string): boolean {
  return pathname === href || pathname.startsWith(`${href}/`)
}
