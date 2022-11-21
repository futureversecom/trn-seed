module.exports = {
	printWidth: 80,
	semi: true,
	useTabs: true,
	singleQuote: false,
	jsxSingleQuote: false,
	quoteProps: "consistent",
	trailingComma: "es5",
	bracketSpacing: true,
	bracketSameLine: false,
	arrowParens: "always",
	overrides: [
		{
			files: ["*.yml", "*.yaml", "*.json", ".eslintrc", ".prettierrc"],
			options: {
				useTabs: false,
				tabWidth: 2,
			},
		},
	],
	importOrder: ["<THIRD_PARTY_MODULES>", "^@/(.*)$", "^[./]"],
	importOrderSeparation: true,
	importOrderSortSpecifiers: true,
	importOrderCaseInsensitive: true,
	plugins: ["@trivago/prettier-plugin-sort-imports"],
};