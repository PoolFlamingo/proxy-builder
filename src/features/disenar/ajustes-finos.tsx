import { RotateCcw } from "lucide-react";
import type { FineTuning } from "@/bindings/FineTuning";
import { FilaDeslizador, FilaInterruptor } from "@/components/campos";
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
		f.cardRadius !== undefined
	);
}

type Posicion = "title" | "cost" | "typeLine" | "emblem" | "rules" | "pt" | "footer";

/**
 * Los ajustes finos de una carta o de una colección: la altura de cada
 * texto, lo opaco del fondo de la caja de reglas y el radio de las esquinas
 * de la ilustración y de la carta.
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
	return (
		<div className="flex flex-col gap-2.5">
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
