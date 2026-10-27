import { useMemo, useState } from "react";
import type { Destino } from "@/bindings/Destino";
import type { Filtro } from "@/bindings/Filtro";
import type { FormaExcavar } from "@/bindings/FormaExcavar";
import type { IdCarta } from "@/bindings/IdCarta";
import type { RestoExcavar } from "@/bindings/RestoExcavar";
import { CampoNumero, Fila, Selector } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { useMesa } from "@/stores/mesa-store";
import { destino, destinoComun, hacer } from "../acciones";
import { caraVisible, ImagenCarta } from "../imagen-carta";
import { usePantalla } from "../pantalla";

type Preset = "descubrir" | "cascada" | "hasta" | "revelarN" | "jodah" | "sunbird" | "medida";

const FILTRO_VACIO: Filtro = {
	noTierra: false,
	supertipo: null,
	tipo: null,
	cmc: null,
	colores: [],
	nombre: null,
};

/**
 * Lo que manda cada preset de «Revelar y lanzar» (`capturas/playgroup/64–69`):
 * la forma, el filtro, cuántas aciertos parar y a dónde va el resto.
 */
export function excavacion(
	preset: Preset,
	n: number,
	tipo: string,
	pararTras: number,
	resto: RestoExcavar,
	medida: { noTierra: boolean; legendaria: boolean },
): { forma: FormaExcavar; filtro: Filtro; pararTras: number; resto: RestoExcavar } {
	switch (preset) {
		case "descubrir":
			return {
				forma: { tipo: "hastaQueCoincida" },
				filtro: { ...FILTRO_VACIO, noTierra: true, cmc: ["menorOIgual", n] },
				pararTras: 1,
				resto: "abajoAlAzar",
			};
		case "cascada":
			return {
				forma: { tipo: "hastaQueCoincida" },
				filtro: { ...FILTRO_VACIO, noTierra: true, cmc: ["menor", n] },
				pararTras: 1,
				resto: "abajoAlAzar",
			};
		case "hasta":
			return {
				forma: { tipo: "hastaQueCoincida" },
				filtro: { ...FILTRO_VACIO, tipo: tipo.trim() || null },
				pararTras,
				resto,
			};
		case "revelarN":
			return { forma: { tipo: "revelarN", n }, filtro: FILTRO_VACIO, pararTras: 1, resto };
		case "jodah":
			return {
				forma: { tipo: "hastaQueCoincida" },
				filtro: { ...FILTRO_VACIO, noTierra: true, supertipo: "Legendary", cmc: ["menor", n] },
				pararTras: 1,
				resto: "abajoAlAzar",
			};
		case "sunbird":
			return {
				forma: { tipo: "revelarN", n },
				filtro: FILTRO_VACIO,
				pararTras: 1,
				resto: "abajoAlAzar",
			};
		case "medida":
			return {
				forma: { tipo: "hastaQueCoincida" },
				filtro: {
					...FILTRO_VACIO,
					noTierra: medida.noTierra,
					supertipo: medida.legendaria ? "Legendary" : null,
					tipo: tipo.trim() || null,
					cmc: n > 0 ? ["menorOIgual", n] : null,
				},
				pararTras,
				resto,
			};
	}
}

type DestinoRevelada = "resto" | "lanzar" | "mano" | "campo";

/**
 * **Revelar y lanzar**: un solo diálogo con los presets (descubrir, cascada,
 * revelar hasta, revelar N, Jodah, Sunbird's) y la excavación a medida.
 * Primero se revela (todos lo ven); después se elige qué hace cada carta y
 * a dónde va el resto.
 */
export function RevelarDialog({ presetInicial }: { presetInicial?: Preset }) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo ?? 0);
	const lib = useMesa((s) => s.zonas[`${yo}:biblioteca`]);
	const cartas = useMesa((s) => s.cartas);
	const [preset, setPreset] = useState<Preset>(presetInicial ?? "descubrir");
	const [n, setN] = useState(3);
	const [tipo, setTipo] = useState("Creature");
	const [pararTras, setPararTras] = useState(1);
	const [resto, setResto] = useState<RestoExcavar>("abajoAlAzar");
	const [medida, setMedida] = useState({ noTierra: true, legendaria: false });
	// Las veces que se repite (Apex Devastator: cascada cuatro veces) y por cuál va.
	const [veces, setVeces] = useState(1);
	const [vez, setVez] = useState(1);
	const [reveladas, setReveladas] = useState<IdCarta[] | null>(null);
	const [destinos, setDestinos] = useState<Record<IdCarta, DestinoRevelada>>({});
	const plan = excavacion(preset, n, tipo, pararTras, resto, medida);
	const repetible = preset === "descubrir" || preset === "cascada" || preset === "jodah";
	const total = repetible ? veces : 1;

	// Lo revelado: lo de arriba de la biblioteca que todos ven.
	const ahora = useMemo(
		() =>
			(lib?.visibles ?? [])
				.filter(([, id]) => cartas[id]?.revelada)
				.sort(([a], [b]) => a - b)
				.map(([, id]) => id),
		[lib, cartas],
	);
	const cerrar = () => usePantalla.getState().abrir(null);

	const excavar = () =>
		hacer({
			que: "excavar",
			forma: plan.forma,
			filtro: plan.filtro,
			pararTras: plan.pararTras,
		});
	const revelar = async () => {
		setVez(1);
		if (await excavar()) setReveladas([]);
	};
	const lista = reveladas !== null ? ahora : [];
	// La que coincide (la última revelada en «hasta que»), elegida para lanzar.
	const elegidaPorDefecto =
		plan.forma.tipo === "hastaQueCoincida" ? lista[lista.length - 1] : undefined;
	const dest = (id: IdCarta): DestinoRevelada =>
		destinos[id] ?? (id === elegidaPorDefecto ? "lanzar" : "resto");
	const terminar = async () => {
		const elegidas: [IdCarta, Destino][] = [];
		const quedan: IdCarta[] = [];
		for (const id of lista) {
			const d = dest(id);
			if (d === "resto") quedan.push(id);
			else
				elegidas.push([
					id,
					d === "lanzar"
						? destinoComun("pila")
						: d === "mano"
							? destino("mano")
							: destino("campo", { tipo: "arriba" }, yo),
				]);
		}
		const ok = await hacer({ que: "resolverExcavacion", elegidas, resto: quedan, a: plan.resto });
		if (!ok) return;
		// Otra vez, con el mismo valor: lo de antes ya está abajo o en la pila.
		if (vez < total) {
			setVez(vez + 1);
			setDestinos({});
			if (!(await excavar())) cerrar();
			return;
		}
		cerrar();
	};

	const PRESETS: { valor: Preset; etiqueta: string }[] = [
		{ valor: "descubrir", etiqueta: t("Descubrir N") },
		{ valor: "cascada", etiqueta: t("Cascada (coste menor que N)") },
		{ valor: "hasta", etiqueta: t("Revelar hasta un tipo") },
		{ valor: "revelarN", etiqueta: t("Revelar N") },
		{ valor: "jodah", etiqueta: t("Jodah (legendaria, coste menor que N)") },
		{ valor: "sunbird", etiqueta: t("Sunbird's Invocation (revelar N)") },
		{ valor: "medida", etiqueta: t("A medida") },
	];

	return (
		<Dialog open onOpenChange={(o) => !o && reveladas === null && cerrar()}>
			<DialogContent className="sm:max-w-3xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Revelar y lanzar")}</DialogTitle>
					<DialogDescription>
						{reveladas === null
							? t("Elige cómo se revela. Todos verán lo revelado.")
							: total > 1
								? t(
										"Revelación {vez} de {veces}. Elige qué haces con cada carta; el resto va abajo.",
										{
											vez,
											veces: total,
										},
									)
								: t("Elige qué haces con cada carta revelada; el resto va a su sitio.")}
					</DialogDescription>
				</DialogHeader>
				{reveladas === null ? (
					<div className="flex flex-col gap-2.5">
						<Fila etiqueta={t("Efecto")}>
							<Selector<Preset>
								valor={preset}
								onChange={setPreset}
								opciones={PRESETS}
								ancho="w-72"
							/>
						</Fila>
						{preset !== "hasta" && (
							<Fila
								etiqueta={
									preset === "cascada" || preset === "jodah"
										? t("Coste de maná de la carta (N)")
										: "N"
								}
							>
								<CampoNumero valor={n} onChange={(v) => setN(v ?? 1)} min={0} max={30} paso={1} />
							</Fila>
						)}
						{repetible && (
							<Fila etiqueta={t("Veces")}>
								<CampoNumero
									valor={veces}
									onChange={(v) => setVeces(v ?? 1)}
									min={1}
									max={20}
									paso={1}
								/>
							</Fila>
						)}
						{(preset === "hasta" || preset === "medida") && (
							<>
								<Fila etiqueta={t("Tipo o subtipo (en inglés)")}>
									<Input
										value={tipo}
										onChange={(e) => setTipo(e.target.value)}
										className="h-7 w-48 text-xs"
									/>
								</Fila>
								<Fila etiqueta={t("Parar tras cuántas")}>
									<CampoNumero
										valor={pararTras}
										onChange={(v) => setPararTras(v ?? 1)}
										min={1}
										max={10}
										paso={1}
									/>
								</Fila>
							</>
						)}
						{preset === "medida" && (
							<>
								<Fila etiqueta={t("Que no sea tierra")}>
									<Switch
										checked={medida.noTierra}
										onCheckedChange={(v) => setMedida({ ...medida, noTierra: v })}
									/>
								</Fila>
								<Fila etiqueta={t("Legendaria")}>
									<Switch
										checked={medida.legendaria}
										onCheckedChange={(v) => setMedida({ ...medida, legendaria: v })}
									/>
								</Fila>
							</>
						)}
						{(preset === "hasta" || preset === "revelarN" || preset === "medida") && (
							<Fila etiqueta={t("El resto va")}>
								<Selector<RestoExcavar>
									valor={resto}
									onChange={setResto}
									opciones={[
										{ valor: "abajoAlAzar", etiqueta: t("Abajo, en orden al azar") },
										{ valor: "abajo", etiqueta: t("Abajo, en este orden") },
										{ valor: "cementerio", etiqueta: t("Al cementerio") },
										{ valor: "exilio", etiqueta: t("Al exilio") },
										{ valor: "arriba", etiqueta: t("Se queda arriba") },
									]}
								/>
							</Fila>
						)}
					</div>
				) : (
					<ScrollArea viewportClassName="max-h-[55vh]">
						<div className="flex flex-wrap gap-2.5 p-1">
							{lista.map((id) => {
								const c = cartas[id];
								if (!c) return null;
								return (
									<div key={id} className="flex w-32 flex-col gap-1">
										<div className="h-[178px] overflow-hidden rounded-[5%]">
											<ImagenCarta imagen={caraVisible(c)} ancho={244} nombre={c.origen?.nombre} />
										</div>
										<div className="grid grid-cols-2 gap-0.5">
											{(
												[
													["lanzar", t("Lanzar")],
													["mano", t("Mano")],
													["campo", t("Campo")],
													["resto", t("Rechazar")],
												] as [DestinoRevelada, string][]
											).map(([d, nombre]) => (
												<Button
													key={d}
													size="sm"
													variant={dest(id) === d ? "default" : "outline"}
													className="h-6 px-1 text-[10px]"
													onClick={() => setDestinos({ ...destinos, [id]: d })}
												>
													{nombre}
												</Button>
											))}
										</div>
									</div>
								);
							})}
							{lista.length === 0 && (
								<p className="p-4 text-xs text-muted-foreground">{t("No se ha revelado nada.")}</p>
							)}
						</div>
					</ScrollArea>
				)}
				<DialogFooter>
					{reveladas === null ? (
						<>
							<Button variant="outline" onClick={cerrar}>
								{t("Cancelar")}
							</Button>
							<Button onClick={() => void revelar()}>{t("Revelar")}</Button>
						</>
					) : (
						<Button onClick={() => void terminar()}>
							{vez < total ? t("Siguiente revelación") : t("Hecho")}
						</Button>
					)}
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
