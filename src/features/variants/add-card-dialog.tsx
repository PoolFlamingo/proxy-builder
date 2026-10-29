import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { CardSummary } from "@/bindings/CardSummary";
import { CampoNumero } from "@/components/campos";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { scryfallDe } from "@/features/disenar/grupos";
import { cartaPorId } from "@/features/preview/acciones-carta";
import { RaizScroll } from "@/features/preview/visibilidad";
import { insertar, nuevoId, quitar } from "@/lib/deck-ops";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { FiltrosBusqueda, idiomaParaRust } from "./filtros";
import { RejillaCartas } from "./rejilla-cartas";
import { traerCarta } from "./traer";

/**
 * «Añadir carta»: buscar en Scryfall (con su sintaxis o por nombre, con
 * autocompletado), elegir impresión y cantidad, y añadirla al final o en la
 * casilla desde la que se abrió. Con `cambiar` (el id de una carta del deck)
 * es «Cambiar carta»: la elegida sustituye a esa copia o a todas las de su
 * misma carta, y no se elige cantidad.
 */
export function AddCardDialog() {
	const dialogo = useUi((s) => s.dialogo);
	const abierto = dialogo?.tipo === "anadir";
	const en = dialogo?.tipo === "anadir" ? dialogo.en : undefined;
	const hueco = dialogo?.tipo === "anadir" ? dialogo.hueco : undefined;
	const cambiar = dialogo?.tipo === "anadir" ? dialogo.cambiar : undefined;
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{abierto && <Contenido en={en} hueco={hueco} cambiar={cambiar} />}
		</Dialog>
	);
}

function Contenido({ en, hueco, cambiar }: { en?: number; hueco?: string; cambiar?: string }) {
	const { t, mensaje } = useLang();
	const vieja = cambiar ? cartaPorId(cambiar) : undefined;
	const viejaScryfall = vieja ? scryfallDe(vieja) : null;
	// Las copias de la misma carta (con esta), para «cambiar todas».
	const copias = useDeck((s) =>
		viejaScryfall ? (s.deck?.cards.filter((c) => scryfallDe(c) === viejaScryfall).length ?? 0) : 0,
	);
	const idiomaDeck = useUi((s) => s.lang);
	const [q, setQ] = useState("");
	const [sugerencias, setSugerencias] = useState<string[]>([]);
	const [lang, setLang] = useState(idiomaDeck === "en" ? "any" : idiomaDeck);
	const [altaRes, setAltaRes] = useState(false);
	const [resultados, setResultados] = useState<CardSummary[] | null>(null);
	const [elegida, setElegida] = useState<CardSummary | null>(null);
	const [cantidad, setCantidad] = useState(1);
	const [buscando, setBuscando] = useState(false);
	const [anadiendo, setAnadiendo] = useState(false);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);

	useEffect(() => {
		const texto = q.trim();
		if (texto.length < 2) return setSugerencias([]);
		let vigente = true;
		const id = setTimeout(() => {
			call("card_autocomplete", { q: texto })
				.then((s) => vigente && setSugerencias(s.slice(0, 8)))
				.catch(() => vigente && setSugerencias([]));
		}, 200);
		return () => {
			vigente = false;
			clearTimeout(id);
		};
	}, [q]);

	const buscar = async (texto: string, exacto = false) => {
		const consulta = exacto ? `!"${texto.replace(/"/g, "")}"` : texto.trim();
		if (!consulta) return;
		setSugerencias([]);
		setBuscando(true);
		setElegida(null);
		try {
			const r = exacto
				? await call("card_prints", {
						name: texto,
						lang: idiomaParaRust(lang),
						highresOnly: altaRes,
					})
				: await call("card_search", {
						q: consulta,
						lang: idiomaParaRust(lang),
						highresOnly: altaRes,
					});
			setResultados(r);
			if (r.length === 1) setElegida(r[0]);
		} catch (e) {
			setResultados([]);
			const codigo = (e as { code?: string }).code;
			if (codigo !== "NotFound") toast.error(mensaje(e));
		} finally {
			setBuscando(false);
		}
	};

	const anadir = async (c: CardSummary | null = elegida) => {
		if (!c) return;
		setAnadiendo(true);
		try {
			const base = await traerCarta(c);
			const nuevas = Array.from({ length: Math.max(1, cantidad) }, (_, i) =>
				i === 0 ? base : { ...base, id: nuevoId() },
			);
			// Un hueco en blanco se sustituye: la carta ocupa su posición.
			useDeck.getState().setCartas((cs) => insertar(hueco ? quitar(cs, [hueco]) : cs, nuevas, en));
			useUi
				.getState()
				.anunciar(t("{n} × {carta} añadida", { n: nuevas.length, carta: base.meta.name }));
			useUi.getState().abrir(null);
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setAnadiendo(false);
		}
	};

	/**
	 * Cambia la carta del deck por `c`: la copia (`todas`: también las demás
	 * de su misma carta) se queda con su sitio, lo fijado, la marca y un
	 * reverso propio; lo demás (retoques, diseño) era de la carta anterior.
	 */
	const sustituir = async (todas: boolean, c: CardSummary | null = elegida) => {
		if (!c || !vieja) return;
		setAnadiendo(true);
		try {
			const base = await traerCarta(c);
			useDeck.getState().setCartas((cs) =>
				cs.map((x) =>
					x.id === vieja.id || (todas && viejaScryfall && scryfallDe(x) === viejaScryfall)
						? {
								...base,
								id: x.id,
								locked: x.locked,
								marca: x.marca,
								back: x.back.kind === "custom" ? x.back : base.back,
							}
						: x,
				),
			);
			useUi.getState().anunciar(t("Cambiada por {carta}", { carta: base.meta.name }));
			useUi.getState().abrir(null);
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setAnadiendo(false);
		}
	};

	return (
		<DialogContent className="flex max-h-[85vh] flex-col sm:max-w-4xl" ocupado={anadiendo}>
			<DialogHeader>
				<DialogTitle>{vieja ? t("Cambiar carta") : t("Añadir carta")}</DialogTitle>
				<DialogDescription>
					{vieja
						? t("Busca la carta que sustituye a esta. Doble clic para cambiar solo esta copia.")
						: t(
								"Busca por nombre o con la sintaxis de Scryfall (t:dragon c:r). Doble clic para añadir.",
							)}
				</DialogDescription>
			</DialogHeader>
			<div className="flex flex-col gap-2.5" data-filtro="">
				<div className="relative flex gap-1.5">
					<div className="relative flex-1">
						<Icono
							icon="lucide:search"
							className="pointer-events-none absolute top-2 left-2 size-4 text-muted-foreground"
						/>
						<Input
							autoFocus
							value={q}
							onChange={(e) => setQ(e.target.value)}
							onKeyDown={(e) => e.key === "Enter" && void buscar(q)}
							placeholder={t("Nombre de la carta…")}
							className="h-8 pl-8 text-xs"
						/>
						{sugerencias.length > 0 && (
							<div className="absolute inset-x-0 top-9 z-10 flex flex-col rounded-md border bg-popover p-1 shadow-lg">
								{sugerencias.map((s) => (
									<button
										key={s}
										type="button"
										className="rounded-sm px-2 py-1 text-left text-xs hover:bg-accent"
										onClick={() => {
											setQ(s);
											void buscar(s, true);
										}}
									>
										{s}
									</button>
								))}
							</div>
						)}
					</div>
					<Button size="sm" onClick={() => buscar(q)} disabled={buscando || !q.trim()}>
						{buscando ? (
							<Icono icon="lucide:loader-2" className="animate-spin" />
						) : (
							<Icono icon="lucide:search" />
						)}
						{t("Buscar")}
					</Button>
				</div>
				<FiltrosBusqueda lang={lang} onLang={setLang} altaRes={altaRes} onAltaRes={setAltaRes} />
			</div>
			<DialogCuerpo className="min-h-64" onViewportChange={setViewport}>
				<RaizScroll.Provider value={viewport}>
					{resultados === null ? (
						<p className="py-16 text-center text-xs text-muted-foreground">
							{t("Escribe un nombre y pulsa Enter.")}
						</p>
					) : resultados.length === 0 ? (
						<p className="py-16 text-center text-xs text-muted-foreground">
							{t("Sin resultados.")}
						</p>
					) : (
						<RejillaCartas
							cartas={resultados}
							elegidas={new Set(elegida ? [elegida.id] : [])}
							onElegir={(c) => setElegida(c)}
							onDoble={(c) => {
								setElegida(c);
								void (vieja ? sustituir(false, c) : anadir(c));
							}}
						/>
					)}
				</RaizScroll.Provider>
			</DialogCuerpo>
			<DialogFooter className="items-center">
				<span className="mr-auto truncate text-xs text-muted-foreground">
					{elegida
						? `${elegida.printedName ?? elegida.name} · ${elegida.set.toUpperCase()} ${elegida.collectorNumber}`
						: resultados
							? t("{n} resultados", { n: resultados.length })
							: ""}
				</span>
				{vieja ? (
					<>
						{copias > 1 && (
							<Button
								size="sm"
								variant="outline"
								onClick={() => sustituir(true)}
								disabled={!elegida || anadiendo}
							>
								{t("Cambiar todas las copias")}
							</Button>
						)}
						<Button size="sm" onClick={() => sustituir(false)} disabled={!elegida || anadiendo}>
							{anadiendo ? (
								<Icono icon="lucide:loader-2" className="animate-spin" />
							) : (
								<Icono icon="lucide:repeat-2" />
							)}
							{t("Cambiar")}
						</Button>
					</>
				) : (
					<>
						<span className="text-xs">{t("Copias")}</span>
						<CampoNumero
							valor={cantidad}
							onChange={(v) => setCantidad(Math.max(1, Math.round(v ?? 1)))}
							min={1}
							max={99}
							paso={1}
							ancho="w-16"
						/>
						<Button size="sm" onClick={() => anadir()} disabled={!elegida || anadiendo}>
							{anadiendo ? (
								<Icono icon="lucide:loader-2" className="animate-spin" />
							) : (
								<Icono icon="lucide:plus" />
							)}
							{t("Añadir")}
						</Button>
					</>
				)}
			</DialogFooter>
		</DialogContent>
	);
}
