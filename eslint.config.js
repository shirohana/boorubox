import js from '@eslint/js'
import stylistic from '@stylistic/eslint-plugin'
import tailwindcss from 'eslint-plugin-better-tailwindcss'
import tseslint from 'typescript-eslint'
import svelte from 'eslint-plugin-svelte'
import globals from 'globals'
import svelteConfig from './packages/app/svelte.config.js'

export default tseslint.config(
  {
    ignores: [
      '**/node_modules/',
      '**/dist/',
      '**/build/',
      '**/.svelte-kit/',
      '**/src-tauri/target/',
      '**/src-tauri/gen/',
      // shadcn-svelte copy-in components: upstream code, updated by its CLI
      'packages/app/src/lib/components/ui/',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs.recommended,
  // Formatting is an ESLint concern here (no Prettier): `eslint --fix` is the formatter.
  stylistic.configs.customize({
    indent: 2,
    quotes: 'single',
    semi: false,
    commaDangle: 'always-multiline',
    braceStyle: '1tbs',
    arrowParens: true,
  }),
  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
    },
    rules: {
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      '@stylistic/max-len': ['warn', { code: 100, ignoreStrings: true, ignoreUrls: true, ignoreTemplateLiterals: true }],
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
    languageOptions: {
      parserOptions: {
        projectService: true,
        extraFileExtensions: ['.svelte'],
        parser: tseslint.parser,
        svelteConfig,
      },
    },
    rules: {
      // Template indentation belongs to the Svelte plugin;
      // @stylistic/indent only understands the script block.
      '@stylistic/indent': 'off',
      'svelte/indent': ['error', { indent: 2 }],
    },
  },
  {
    files: ['packages/app/**/*.{svelte,ts,js}'],
    plugins: { 'better-tailwindcss': tailwindcss },
    rules: {
      ...tailwindcss.configs.recommended.rules,
      'better-tailwindcss/no-unknown-classes': 'off',
      'better-tailwindcss/enforce-consistent-line-wrapping': ['warn', {
        printWidth: 100, indent: 2, preferSingleLine: true,
      }],
    },
    settings: {
      'better-tailwindcss': {
        entryPoint: 'packages/app/src/app.css',
        messageStyle: 'compact',
      },
    },
  },
  {
    files: ['packages/extension/**/*.ts'],
    languageOptions: { globals: { chrome: 'readonly' } },
  },
)
