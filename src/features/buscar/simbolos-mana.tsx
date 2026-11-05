import { Fragment } from "react";
import { Icono } from "@/components/icono";
import { partirSimbolos } from "./consulta";

const COLOR: Record<string, string> = {
	W: "#f8f3d4",
	U: "#a9d1ec",
	B: "#b8aea8",
	R: "#ec9e7c",
	G: "#9ccb9f",
};
const NEUTRO = "#cfc9c5";

/** El fondo de un símbolo: su color o, si es híbrido (`W/U`), los dos a medias. */
function fondo(simbolo: string): string {
	const colores = simbolo.split("/").flatMap((p) => (COLOR[p] ? [COLOR[p]] : []));
	if (colores.length === 2) return `linear-gradient(135deg, ${colores[0]} 50%, ${colores[1]} 50%)`;
	return colores[0] ?? NEUTRO;
}

/** Un símbolo de Scryfall sin las llaves (`T`, `2`, `W/U`…), a la medida del texto. */
export function SimboloMana({ simbolo }: { simbolo: string }) {
	return (
		<span
			role="img"
			aria-label={`{${simbolo}}`}
			className="mx-px inline-flex size-[1.25em] items-center justify-center rounded-full align-[-0.2em] text-[0.8em] font-bold text-black shadow-[0_0_0_0.5px_rgba(0,0,0,0.45)]"
			style={{ background: fondo(simbolo) }}
		>
			{simbolo === "T" ? (
				<Icono icon="lucide:rotate-cw" className="size-[0.85em]" />
			) : simbolo === "Q" ? (
				<Icono icon="lucide:rotate-ccw" className="size-[0.85em]" />
			) : simbolo.includes("/") ? (
				""
			) : (
				simbolo
			)}
		</span>
	);
}

/** Un texto de reglas o un coste, con sus símbolos pintados. */
export function TextoConSimbolos({ texto }: { texto: string }) {
	return (
		<>
			{partirSimbolos(texto).map((t, i) => (
				// El mismo texto parte siempre igual: su posición es una clave estable.
				<Fragment key={i}>{t.simbolo ? <SimboloMana simbolo={t.texto} /> : t.texto}</Fragment>
			))}
		</>
	);
}
