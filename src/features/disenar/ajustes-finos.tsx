import { RotateCcw } from "lucide-react";
import type { FineTuning } from "@/bindings/FineTuning";
import { Fila, FilaDeslizador, FilaInterruptor, Selector } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";

/** Sin ajustes: todo como viene en el marco. */
export const AJUSTES_CERO: FineTuning = {
	title: 0,
	cost: 0,
	typeLine: 0,
	emblem: 0,
	rules: 0,
	pt: 0,
	footer: 0,
	rulesBoxOpacity: 0,
};

/** ¿Hay algo tocado? */
export function hayAjustes(f: FineTuning | undefined): boolean {
	if (!f) return false;
	return (
		Object.entries(AJUSTES_CERO).some(([k, v]) => f[k as keyof FineTuning] !== v) ||
		f.artRadius !== undefined ||
		f.artFade !== undefined ||
		f.artFadeExtension !== undefined ||
		f.rulesBoxHeight !== undefined ||
		f.rulesBoxDynamic !== undefined ||
		f.cardRadius !== undefined ||
		TAMANOS.some((k) => f[k] !== undefined)
	);
}

/** El tamaño de cada texto (en proporción al del marco; sin él, el del marco). */
const TAMANOS = [
	"titleSize",
	"typeSize",
	"rulesSize",
	"ptSize",
	"footerSize",
	"rulesLineSpacing",
	"footerLineSpacing",
] as const;
type Tamano = (typeof TAMANOS)[number];

/** La altura de la caja de reglas: la del marco, dinámica o fija. */
type AlturaCaja = "marco" | "dinamica" | "fija";

const alturaDe = (f: FineTuning): AlturaCaja =>
	f.rulesBoxDynamic === undefined ? "marco" : f.rulesBoxDynamic ? "dinamica" : "fija";

type Posicion = "title" | "cost" | "typeLine" | "emblem" | "rules" | "pt" | "footer";

/**
 * Los ajustes finos de una carta o de una colección: el tamaño y la altura
 * de cada texto, lo opaco del fondo de la caja de reglas y el radio de las
 * esquinas de la ilustración y de la carta y el difuminado de abajo de la
 * ilustración.
 */
export function AjustesFinos({
	valor,
	onChange,
}: {
	valor: FineTuning;
	onChange: (f: FineTuning) => void;
}) {
	const { t } = useLang();
	const poner = (c: Partial<FineTuning>) => onChange({ ...valor, ...c });
	const posiciones: [Posicion, string][] = [
		["title", t("Título")],
		["cost", t("Coste de maná")],
		["typeLine", t("Línea de tipo")],
		["emblem", t("Emblema")],
		["rules", t("Texto de reglas")],
		["pt", t("Fuerza/resistencia")],
		["footer", t("Pie")],
	];
	const tamanos: [Exclude<Tamano, "rulesLineSpacing" | "footerLineSpacing">, string][] = [
		["titleSize", t("Título")],
		["typeSize", t("Línea de tipo")],
		["rulesSize", t("Texto de reglas")],
		["ptSize", t("Fuerza/resistencia")],
		["footerSize", t("Pie")],
	];
	return (
		<div className="flex flex-col gap-2.5">
			<p className="text-xs font-semibold">{t("Tamaño del texto e interlineado")}</p>
			<p className="text-[11px] text-muted-foreground">
				{t(
					"Cada texto en proporción al del marco (100 % = como viene). Más pequeñas, las reglas dejan más sitio al arte en los marcos con la caja que crece.",
				)}
			</p>
			<FilaInterruptor
				etiqueta={t("Reglas: tamaño automático")}
				ayuda={t(
					"La letra baja lo justo para que quepa y, en los marcos con la caja que crece, un poco antes de comerse el arte. Apagado, las reglas van al tamaño que elijas y la caja crece lo que haga falta.",
				)}
				valor={valor.rulesSize === undefined}
				onChange={(v) => poner({ rulesSize: v ? undefined : 1 })}
			/>
			{tamanos
				.filter(([k]) => k !== "rulesSize" || valor.rulesSize !== undefined)
				.map(([k, etiqueta]) => (
					<FilaDeslizador
						key={k}
						etiqueta={etiqueta}
						valor={Math.round((valor[k] ?? 1) * 100)}
						min={50}
						max={160}
						paso={5}
						sufijo="%"
						// 100 % es el del marco: no se guarda (salvo en las reglas,
						// donde sin valor es «automático»).
						onChange={(v) => poner({ [k]: v === 100 && k !== "rulesSize" ? undefined : v / 100 })}
					/>
				))}
			<FilaDeslizador
				etiqueta={t("Interlineado de las reglas")}
				valor={Math.round((valor.rulesLineSpacing ?? 1) * 100)}
				min={70}
				max={160}
				paso={5}
				sufijo="%"
				onChange={(v) => poner({ rulesLineSpacing: v === 100 ? undefined : v / 100 })}
			/>
			<FilaDeslizador
				etiqueta={t("Interlineado del pie")}
				valor={Math.round((valor.footerLineSpacing ?? 1) * 100)}
				min={70}
				max={160}
				paso={5}
				sufijo="%"
				onChange={(v) => poner({ footerLineSpacing: v === 100 ? undefined : v / 100 })}
			/>
			<p className="pt-1 text-xs font-semibold">{t("Posición")}</p>
			<p className="text-[11px] text-muted-foreground">
				{t("Altura de cada cosa, en milímetros: en positivo baja y en negativo sube.")}
			</p>
			{posiciones.map(([k, etiqueta]) => (
				<FilaDeslizador
					key={k}
					etiqueta={etiqueta}
					valor={valor[k]}
					min={-3}
					max={3}
					paso={0.05}
					sufijo=" mm"
					onChange={(v) => poner({ [k]: v })}
				/>
			))}
			<p className="pt-1 text-xs font-semibold">{t("Caja de reglas")}</p>
			<p className="text-[11px] text-muted-foreground">
				{t(
					"Cuánto más alta (en positivo, crece hacia arriba y la ilustración se acorta) o más baja (en negativo, la letra baja si hace falta). A 0, como viene. En el marco Balatro no cambia.",
				)}
			</p>
			<Fila
				etiqueta={t("Altura según el texto")}
				ayuda={t(
					"Dinámica: la caja crece o se encoge según el texto y las fichas con poco texto quedan despejadas. Fija: siempre del alto máximo del marco. Según el marco: lo que decida cada marco (en Clásico y Extendido de la casa es fija).",
				)}
			>
				<Selector
					valor={alturaDe(valor)}
					opciones={[
						{ valor: "marco", etiqueta: t("Según el marco") },
						{ valor: "dinamica", etiqueta: t("Dinámica") },
						{ valor: "fija", etiqueta: t("Fija") },
					]}
					onChange={(v) => poner({ rulesBoxDynamic: v === "marco" ? undefined : v === "dinamica" })}
					ancho="w-36"
				/>
			</Fila>
			<FilaDeslizador
				etiqueta={t("Altura de la caja de reglas")}
				valor={valor.rulesBoxHeight ?? 0}
				min={-10}
				max={10}
				paso={0.5}
				sufijo=" mm"
				onChange={(v) => poner({ rulesBoxHeight: v === 0 ? undefined : v })}
			/>
			<FilaDeslizador
				etiqueta={t("Fondo de la caja de reglas más opaco")}
				valor={Math.round(valor.rulesBoxOpacity * 100)}
				min={0}
				max={100}
				sufijo="%"
				onChange={(v) => poner({ rulesBoxOpacity: v / 100 })}
			/>
			<FilaInterruptor
				etiqueta={t("Radio propio de la ilustración")}
				ayuda={t(
					"Las esquinas de abajo de la ventana de la ilustración, en el estilo despejado (también con los marcos Original). A 0 mm, rectas.",
				)}
				valor={valor.artRadius !== undefined}
				onChange={(v) => poner({ artRadius: v ? 6 : undefined })}
			/>
			{valor.artRadius !== undefined && (
				<FilaDeslizador
					etiqueta={t("Radio de la ilustración")}
					valor={valor.artRadius}
					min={0}
					max={20}
					paso={0.5}
					sufijo=" mm"
					onChange={(artRadius) => poner({ artRadius })}
				/>
			)}
			<FilaInterruptor
				etiqueta={t("Difuminado de la ilustración a medida")}
				ayuda={t(
					"Funde con negro el borde de abajo de la ilustración. Sin tocarlo, el del marco (en el extendido, 12 mm con 7 de extensión). A 0 mm, sin difuminado. La extensión hace que la ilustración siga por debajo de su ventana, perdiéndose: se ve donde el marco no la tapa (en el extendido, a los lados de la caja y debajo).",
				)}
				valor={valor.artFade !== undefined}
				onChange={(v) =>
					poner(
						v
							? { artFade: 12, artFadeExtension: 7 }
							: { artFade: undefined, artFadeExtension: undefined },
					)
				}
			/>
			{valor.artFade !== undefined && (
				<>
					<FilaDeslizador
						etiqueta={t("Difuminado de la ilustración")}
						valor={valor.artFade}
						min={0}
						max={25}
						paso={0.5}
						sufijo=" mm"
						onChange={(artFade) => poner({ artFade })}
					/>
					<FilaDeslizador
						etiqueta={t("Extensión del difuminado")}
						valor={valor.artFadeExtension ?? 0}
						min={0}
						max={35}
						paso={0.5}
						sufijo=" mm"
						onChange={(v) => poner({ artFadeExtension: v === 0 ? undefined : v })}
					/>
				</>
			)}
			<FilaInterruptor
				etiqueta={t("Esquinas de la carta a medida")}
				ayuda={t(
					"Sin tocarlo, la carta se ve redondeada y se imprime en ángulo recto. A medida, se ve y se imprime igual: a 0 mm, con esquinas rectas (para fundas).",
				)}
				valor={valor.cardRadius !== undefined}
				onChange={(v) => poner({ cardRadius: v ? 0 : undefined })}
			/>
			{valor.cardRadius !== undefined && (
				<FilaDeslizador
					etiqueta={t("Radio de las esquinas de la carta")}
					valor={valor.cardRadius}
					min={0}
					max={6}
					paso={0.25}
					sufijo=" mm"
					onChange={(cardRadius) => poner({ cardRadius })}
				/>
			)}
			<Button
				size="sm"
				variant="ghost"
				className="self-start"
				disabled={!hayAjustes(valor)}
				onClick={() => onChange(AJUSTES_CERO)}
			>
				<RotateCcw />
				{t("Todo como viene en el marco")}
			</Button>
		</div>
	);
}
