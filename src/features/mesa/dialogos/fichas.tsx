import { Loader2, Search } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import type { CardSummary } from "@/bindings/CardSummary";
import type { OrigenFicha } from "@/bindings/OrigenFicha";
import { CampoNumero } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { urlBusqueda } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { NINGUNA, useMesa } from "@/stores/mesa-store";
import { hacer } from "../acciones";
import { ImagenCarta } from "../imagen-carta";
import { usePantalla } from "../pantalla";

/** Las fichas rápidas (`capturas/playgroup/75`): se buscan en Scryfall por su nombre. */
const RAPIDAS = ["Treasure", "Food", "Clue", "Blood", "Map", "Powerstone"];
/** Las de ayuda: mazmorras, bendición, día y noche, el Anillo… */
const AYUDA = [
	"Lost Mine of Phandelver",
	"Dungeon of the Mad Mage",
	"Tomb of Annihilation",
	"Undercity",
	"The City's Blessing",
	"Day // Night",
	"The Ring",
	"The Monarch",
	"The Initiative",
];

/**
 * **Crear ficha**: las de la sección de fichas del deck, las rápidas, las de
 * ayuda, cualquiera de Scryfall (o una carta de verdad para copiarla) y una
 * libre. Cantidad y «como un montón».
 */
export function FichasDialog() {
	const { t, mensaje } = useLang();
	const yo = useMesa((s) => s.yo ?? 0);
	const delDeck = useMesa((s) => s.jugadores[yo]?.fichas ?? NINGUNA);
	const [cantidad, setCantidad] = useState(1);
	const [monton, setMonton] = useState(false);
	const [q, setQ] = useState("");
	const [soloFichas, setSoloFichas] = useState(true);
	const [resultados, setResultados] = useState<CardSummary[]>([]);
	const [ocupado, setOcupado] = useState(false);
	const [libre, setLibre] = useState({ nombre: "", fuerza: "1", resistencia: "1" });
	const cerrar = () => usePantalla.getState().abrir(null);

	const crear = async (de: OrigenFicha) => {
		const ok = await hacer({ que: "crearFicha", de, cantidad, monton, pos: null });
		if (ok) cerrar();
	};
	/** Busca en Scryfall; `siFalla` se llama en vez de avisar si no hay conexión. */
	const buscar = async (texto: string, fichas: boolean, siFalla?: () => void) => {
		if (!texto.trim()) return;
		setOcupado(true);
		try {
			const consulta = fichas ? `${texto.trim()} (t:token OR t:emblem OR t:dungeon)` : texto.trim();
			setResultados(await call("card_search", { q: consulta, lang: null, highresOnly: false }));
		} catch (e) {
			if (siFalla) siFalla();
			else toast.error(mensaje(e));
		} finally {
			setOcupado(false);
		}
	};
	const deScryfall = async (id: string) => {
		setOcupado(true);
		try {
			const origen = await call("mesa_origen_scryfall", { scryfallId: id });
			await crear({ tipo: "carta", origen });
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(false);
		}
	};

	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-3xl" cambiosSinGuardar={false} ocupado={ocupado}>
				<DialogHeader>
					<DialogTitle>{t("Crear ficha")}</DialogTitle>
					<DialogDescription>
						{t("Sale en tu campo. Pulsa la que quieras crear.")}
					</DialogDescription>
				</DialogHeader>
				<div className="flex items-center gap-2.5 text-xs">
					<span>{t("Cantidad")}</span>
					<CampoNumero
						valor={cantidad}
						onChange={(v) => setCantidad(v ?? 1)}
						min={1}
						max={monton ? 1000 : 100}
						paso={1}
						ariaLabel={t("Cantidad")}
					/>
					<label className="flex items-center gap-1.5">
						<Switch checked={monton} onCheckedChange={setMonton} />
						{t("Como un montón")}
					</label>
				</div>
				<ScrollArea viewportClassName="max-h-[60vh]">
					<div className="flex flex-col gap-2.5 p-1">
						{delDeck.length > 0 && (
							<section className="flex flex-col gap-1.5">
								<h4 className="text-xs font-semibold">{t("Las de tu deck")}</h4>
								<div className="flex flex-wrap gap-2.5">
									{delDeck.map((f, i) => (
										<button
											// Las variantes de una ficha comparten nombre.
											key={`${f.nombre}-${i}`}
											type="button"
											onClick={() => void crear({ tipo: "delDeck", indice: i })}
											className="h-[140px] w-[100px] overflow-hidden rounded-[5%] hover:ring-2 hover:ring-primary"
										>
											<ImagenCarta imagen={f.caras[0] ?? f.dorso} ancho={244} nombre={f.nombre} />
										</button>
									))}
								</div>
							</section>
						)}
						<section className="flex flex-col gap-1.5">
							<h4 className="text-xs font-semibold">{t("Rápidas y de ayuda")}</h4>
							<div className="flex flex-wrap gap-1.5">
								{[...RAPIDAS, ...AYUDA].map((n) => (
									<Button
										key={n}
										size="sm"
										variant="outline"
										className="h-7 text-[11px]"
										onClick={() => {
											setQ(n);
											// Sin Scryfall, la rápida sale igual: genérica, con su nombre.
											void buscar(`!"${n}"`, false, () => {
												toast.info(t("Sin conexión: la ficha sale sin imagen."));
												void crear({
													tipo: "libre",
													nombre: n,
													fuerza: null,
													resistencia: null,
													colores: [],
												});
											});
										}}
									>
										{n}
									</Button>
								))}
							</div>
						</section>
						<section className="flex flex-col gap-1.5" data-filtro="">
							<h4 className="text-xs font-semibold">{t("Buscar en Scryfall")}</h4>
							<div className="flex items-center gap-2.5">
								<Input
									value={q}
									onChange={(e) => setQ(e.target.value)}
									onKeyDown={(e) => e.key === "Enter" && void buscar(q, soloFichas)}
									placeholder={t("Nombre de la ficha o de una carta para copiarla…")}
									className="h-7 text-xs"
								/>
								<label className="flex shrink-0 items-center gap-1.5 text-xs">
									<Switch checked={soloFichas} onCheckedChange={setSoloFichas} />
									{t("Solo fichas")}
								</label>
								<Button size="sm" onClick={() => void buscar(q, soloFichas)} disabled={ocupado}>
									{ocupado ? (
										<Loader2 className="size-3.5 animate-spin" />
									) : (
										<Search className="size-3.5" />
									)}
								</Button>
							</div>
							{resultados.length > 0 && (
								<div className="flex flex-wrap gap-2.5">
									{resultados.slice(0, 60).map((r) => (
										<button
											key={r.id}
											type="button"
											disabled={ocupado}
											onClick={() => void deScryfall(r.id)}
											className="w-[100px] overflow-hidden rounded-[5%] hover:ring-2 hover:ring-primary"
										>
											{r.thumb ? (
												<img src={urlBusqueda(r.thumb)} alt={r.name} loading="lazy" />
											) : (
												r.name
											)}
										</button>
									))}
								</div>
							)}
						</section>
						<section className="flex flex-col gap-1.5">
							<h4 className="text-xs font-semibold">{t("Una libre")}</h4>
							<div className="flex items-center gap-2.5">
								<Input
									value={libre.nombre}
									onChange={(e) => setLibre({ ...libre, nombre: e.target.value })}
									placeholder={t("Nombre")}
									maxLength={60}
									className="h-7 text-xs"
								/>
								<Input
									value={libre.fuerza}
									onChange={(e) => setLibre({ ...libre, fuerza: e.target.value })}
									className="h-7 w-14 text-center text-xs"
								/>
								<span>/</span>
								<Input
									value={libre.resistencia}
									onChange={(e) => setLibre({ ...libre, resistencia: e.target.value })}
									className="h-7 w-14 text-center text-xs"
								/>
								<Button
									size="sm"
									disabled={!libre.nombre.trim()}
									onClick={() =>
										void crear({
											tipo: "libre",
											nombre: libre.nombre.trim(),
											fuerza: libre.fuerza.trim() || null,
											resistencia: libre.resistencia.trim() || null,
											colores: [],
										})
									}
								>
									{t("Crear")}
								</Button>
							</div>
						</section>
					</div>
				</ScrollArea>
			</DialogContent>
		</Dialog>
	);
}
