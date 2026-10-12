/**
 * Lo que se le pasa a CADA fichero que entra en un commit (patrón de judlex).
 *
 * 🛑 Solo toca lo que está en el índice: aquí se ARREGLA (formatear, aplicar
 * los arreglos seguros de Biome) y lint-staged vuelve a añadir el fichero; el
 * `pre-commit` COMPRUEBA el proyecto entero.
 */
const config = {
	"*.{ts,tsx,js,jsx,mjs,cjs}": ["biome check --write --no-errors-on-unmatched"],
	"!(pnpm-lock).{json,html}": ["biome format --write --no-errors-on-unmatched"],
	// 🛑 `rustfmt` a pelo sobre los ficheros del índice, NUNCA `cargo fmt`, que
	// formatea el crate entero y metería en el commit ficheros que nadie tocó.
	// `--edition 2021` no es opcional: sin ella rustfmt asume la 2015 y deja el
	// fichero como estaba sin decir nada.
	"src-tauri/**/*.rs": ["rustfmt --edition 2021"],
};

export default config;
