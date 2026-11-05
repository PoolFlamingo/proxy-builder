import { useState } from "react";
import type { ComparadorCoste } from "@/bindings/ComparadorCoste";
import type { ModoColores } from "@/bindings/ModoColores";
import type { UnicasCartas } from "@/bindings/UnicasCartas";
import { CampoNumero, type Opcion, Selector } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useIdiomasFiltro } from "@/features/variants/filtros";
import { cn } from "@/lib/utils";
import { useBusqueda } from "@/stores/busqueda-store";
import { cuantosFiltros } from "./consulta";
import { SimboloMana } from "./simbolos-mana";

const COLORES = [
	{ id: "w", simbolo: "W", nombre: "Blanco" },
	{ id: "u", simbolo: "U", nombre: "Azul" },
	{ id: "b", simbolo: "B", nombre: "Negro" },
	{ id: "r", simbolo: "R", nombre: "Rojo" },
	{ id: "g", simbolo: "G", nombre: "Verde" },
	{ id: "c", simbolo: "C", nombre: "Incoloro" },
];

/** Las rarezas de la búsqueda, con su nombre en la pantalla. */
export const RAREZAS = [
	{ id: "common", nombre: "Común" },
	{ id: "uncommon", nombre: "Infrecuente" },
	{ id: "rare", nombre: "Rara" },
	{ id: "mythic", nombre: "Mítica" },
];

/** Nombres propios de los formatos: no se traducen. */
export const FORMATOS: Record<string, string> = {
	standard: "Standard",
	pioneer: "Pioneer",
	modern: "Modern",
	legacy: "Legacy",
	vintage: "Vintage",
	commander: "Commander",
	oathbreaker: "Oathbreaker",
	brawl: "Brawl",
	historic: "Historic",
	pauper: "Pauper",
};

/** Qué se quita y qué se pone al pulsar un valor de una lista de elegidos. */
function alternar(lista: string[], valor: string): string[] {
	return lista.includes(valor) ? lista.filter((x) => x !== valor) : [...lista, valor];
}

/** Lo que se puede afinar de una búsqueda: en el lateral o en una hoja, según el formato. */
export function FiltrosPanel() {
	const { t } = useLang();
	const f = useBusqueda((s) => s.filtros);
	const poner = useBusqueda((s) => s.poner);
	const idiomas = useIdiomasFiltro();
	const [comparadorSinCoste, setComparadorSinCoste] = useState<ComparadorCoste>("igual");

	const modos: Opcion<ModoColores>[] = [
		{ valor: "incluye", etiqueta: t("Incluye estos colores") },
		{ valor: "exactos", etiqueta: t("Exactamente estos colores") },
		{ valor: "identidad", etiqueta: t("Identidad dentro de estos") },
	];
	const tipos: Opcion<string>[] = [
		{ valor: "todos", etiqueta: t("Todos los tipos") },
		{ valor: "creature", etiqueta: t("Criaturas") },
		{ valor: "instant", etiqueta: t("Instantáneos") },
		{ valor: "sorcery", etiqueta: t("Conjuros") },
		{ valor: "artifact", etiqueta: t("Artefactos") },
		{ valor: "enchantment", etiqueta: t("Encantamientos") },
		{ valor: "planeswalker", etiqueta: t("Planeswalkers") },
		{ valor: "battle", etiqueta: t("Batallas") },
		{ valor: "land", etiqueta: t("Tierras") },
	];
	const formatos: Opcion<string>[] = [
		{ valor: "todos", etiqueta: t("Cualquier formato") },
		...Object.entries(FORMATOS).map(([valor, etiqueta]) => ({ valor, etiqueta })),
	];
	const comparadores: Opcion<ComparadorCoste>[] = [
		{ valor: "igual", etiqueta: "=" },
		{ valor: "minimo", etiqueta: "≥" },
		{ valor: "maximo", etiqueta: "≤" },
	];
	const unicas: Opcion<UnicasCartas>[] = [
		{ valor: "cartas", etiqueta: t("Una por carta") },
		{ valor: "arte", etiqueta: t("Una por ilustración") },
		{ valor: "impresiones", etiqueta: t("Todas las impresiones") },
	];

	return (
		<div className="flex flex-col gap-4">
			<Grupo titulo={t("Colores")}>
				<div className="flex flex-wrap gap-1">
					{COLORES.map((c) => {
						const puesto = f.colores.includes(c.id);
						return (
							<Button
								key={c.id}
								size="icon-sm"
								variant={puesto ? "secondary" : "ghost"}
								aria-label={t(c.nombre)}
								aria-pressed={puesto}
								className={cn("text-base", puesto && "ring-2 ring-primary")}
								onClick={() => poner({ colores: alternar(f.colores, c.id) })}
							>
								<SimboloMana simbolo={c.simbolo} />
							</Button>
						);
					})}
				</div>
				<Selector
					valor={f.modoColores}
					opciones={modos}
					onChange={(modoColores) => poner({ modoColores })}
					ancho="w-full"
					ariaLabel={t("Colores")}
				/>
			</Grupo>
			<Grupo titulo={t("Tipo")}>
				<Selector
					valor={f.tipo ?? "todos"}
					opciones={tipos}
					onChange={(v) => poner({ tipo: v === "todos" ? null : v })}
					ancho="w-full"
					ariaLabel={t("Tipo")}
				/>
			</Grupo>
			<Grupo titulo={t("Rareza")}>
				<div className="flex flex-wrap gap-1">
					{RAREZAS.map((r) => {
						const puesta = f.rarezas.includes(r.id);
						return (
							<Button
								key={r.id}
								size="xs"
								variant={puesta ? "default" : "outline"}
								aria-pressed={puesta}
								onClick={() => poner({ rarezas: alternar(f.rarezas, r.id) })}
							>
								{t(r.nombre)}
							</Button>
						);
					})}
				</div>
			</Grupo>
			<Grupo titulo={t("Legal en")}>
				<Selector
					valor={f.formato ?? "todos"}
					opciones={formatos}
					onChange={(v) => poner({ formato: v === "todos" ? null : v })}
					ancho="w-full"
					ariaLabel={t("Legal en")}
				/>
			</Grupo>
			<Grupo titulo={t("Coste de maná")}>
				<div className="flex items-center gap-1.5">
					<Selector
						valor={f.coste?.comparador ?? comparadorSinCoste}
						opciones={comparadores}
						onChange={(comparador) => {
							// Sin número no hay filtro: el comparador espera al primero que se escriba.
							setComparadorSinCoste(comparador);
							if (f.coste) poner({ coste: { ...f.coste, comparador } });
						}}
						ancho="w-16"
						ariaLabel={t("Comparar el coste")}
					/>
					<CampoNumero
						valor={f.coste?.valor ?? null}
						onChange={(v) =>
							poner({
								coste:
									v === null
										? null
										: {
												comparador: f.coste?.comparador ?? comparadorSinCoste,
												valor: Math.max(0, Math.round(v)),
											},
							})
						}
						min={0}
						max={20}
						paso={1}
						ancho="w-20"
						vacioEsNulo
						ariaLabel={t("Coste de maná")}
					/>
				</div>
			</Grupo>
			<Grupo titulo={t("Código del set")}>
				<Input
					value={f.set ?? ""}
					onChange={(e) => poner({ set: e.target.value || null })}
					placeholder="MH3"
					aria-label={t("Código del set")}
					autoCapitalize="characters"
					autoCorrect="off"
					spellCheck={false}
					className="h-7 text-xs uppercase placeholder:normal-case"
				/>
			</Grupo>
			<Grupo titulo={t("Idioma")}>
				<Selector
					valor={f.idioma ?? "any"}
					opciones={idiomas}
					onChange={(v) => poner({ idioma: v === "any" ? null : v })}
					ancho="w-full"
					ariaLabel={t("Idioma")}
				/>
			</Grupo>
			<Grupo titulo={t("Resultados")}>
				<Selector
					valor={f.unicas}
					opciones={unicas}
					onChange={(v) => poner({ unicas: v })}
					ancho="w-full"
					ariaLabel={t("Resultados")}
				/>
			</Grupo>
			{cuantosFiltros(f) > 0 && (
				<Button size="sm" variant="outline" onClick={() => useBusqueda.getState().quitarFiltros()}>
					{t("Quitar los filtros")}
				</Button>
			)}
		</div>
	);
}

function Grupo({ titulo, children }: { titulo: string; children: React.ReactNode }) {
	return (
		<div className="flex flex-col gap-1.5">
			<span className="text-[11px] font-semibold text-muted-foreground">{titulo}</span>
			{children}
		</div>
	);
}
