import { Loader2, Replace, ReplaceAll } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { CardSummary } from "@/bindings/CardSummary";
import type { DeckCard } from "@/bindings/DeckCard";
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
import { cartaPorId } from "@/features/preview/acciones-carta";
import { RaizScroll } from "@/features/preview/visibilidad";
import { urlBusqueda } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { FiltrosBusqueda, idiomaParaRust } from "./filtros";
import { RejillaCartas } from "./rejilla-cartas";
import { traerCarta } from "./traer";

/**
 * «Cambiar variante»: todas las impresiones de la carta (filtros de idioma y
 * alta resolución), una vista grande de la elegida y reemplazar solo esta
 * copia o todas las del mismo nombre.
 */
export function VariantDialog() {
	const dialogo = useUi((s) => s.dialogo);
	const id = dialogo?.tipo === "variante" ? dialogo.id : null;
	const carta = id ? cartaPorId(id) : undefined;
	return (
		<Dialog open={!!carta} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{carta && <Contenido carta={carta} />}
		</Dialog>
	);
}

/** La carta nueva conserva lo que es de la copia: id, fijado, retoques y un reverso elegido. */
function reemplazar(vieja: DeckCard, nueva: DeckCard): DeckCard {
	return {
		...nueva,
		id: vieja.id,
		locked: vieja.locked,
		edits: vieja.edits,
		back: vieja.back.kind === "custom" ? vieja.back : nueva.back,
	};
}

function Contenido({ carta }: { carta: DeckCard }) {
	const { t, mensaje } = useLang();
	const [lang, setLang] = useState(carta.meta.lang ?? "any");
	const [altaRes, setAltaRes] = useState(false);
	const [lista, setLista] = useState<CardSummary[] | null>(null);
	const [elegida, setElegida] = useState<CardSummary | null>(null);
	const [cargando, setCargando] = useState(false);
	const [aplicando, setAplicando] = useState(false);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
	const nombre = carta.meta.name;
	const copias = useDeck((s) => s.deck?.cards.filter((c) => c.meta.name === nombre).length ?? 0);

	useEffect(() => {
		let vigente = true;
		setCargando(true);
		call("card_prints", { name: nombre, lang: idiomaParaRust(lang), highresOnly: altaRes })
			.then((r) => vigente && setLista(r))
			.catch((e) => {
				if (!vigente) return;
				setLista([]);
				if ((e as { code?: string }).code !== "NotFound") toast.error(mensaje(e));
			})
			.finally(() => vigente && setCargando(false));
		return () => {
			vigente = false;
		};
	}, [nombre, lang, altaRes, mensaje]);

	const aplicar = async (todas: boolean) => {
		if (!elegida) return;
		setAplicando(true);
		try {
			const nueva = await traerCarta(elegida);
			useDeck
				.getState()
				.setCartas((cs) =>
					cs.map((c) =>
						c.id === carta.id || (todas && c.meta.name === nombre) ? reemplazar(c, nueva) : c,
					),
				);
			useUi
				.getState()
				.anunciar(
					todas
						? t("Variante cambiada en todas las copias de {carta}", { carta: nombre })
						: t("Variante cambiada"),
				);
			useUi.getState().abrir(null);
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setAplicando(false);
		}
	};

	return (
		<DialogContent className="flex max-h-[88vh] flex-col sm:max-w-5xl" ocupado={aplicando}>
			<DialogHeader>
				<DialogTitle>{t("Cambiar variante de {carta}", { carta: nombre })}</DialogTitle>
				<DialogDescription>
					{carta.meta.set
						? t("Ahora: {set} {num}", {
								set: carta.meta.set.toUpperCase(),
								num: carta.meta.collectorNumber ?? "",
							})
						: ""}
				</DialogDescription>
			</DialogHeader>
			<div data-filtro="">
				<FiltrosBusqueda lang={lang} onLang={setLang} altaRes={altaRes} onAltaRes={setAltaRes} />
			</div>
			<div className="flex min-h-0 flex-1 gap-4">
				<DialogCuerpo className="min-h-72 flex-1" onViewportChange={setViewport}>
					<RaizScroll.Provider value={viewport}>
						{cargando && !lista ? (
							<div className="flex justify-center py-16">
								<Loader2 className="size-5 animate-spin text-muted-foreground" />
							</div>
						) : lista && lista.length === 0 ? (
							<p className="py-16 text-center text-xs text-muted-foreground">
								{t("No hay impresiones con esos filtros.")}
							</p>
						) : (
							<RejillaCartas
								cartas={lista ?? []}
								elegidas={new Set(elegida ? [elegida.id] : [])}
								onElegir={(c) => setElegida(c)}
								onDoble={(c) => {
									setElegida(c);
								}}
							/>
						)}
					</RaizScroll.Provider>
				</DialogCuerpo>
				<div className="hidden w-64 shrink-0 flex-col gap-2 md:flex">
					<div className="aspect-[63/88] w-full overflow-hidden rounded-[4.5%] bg-muted">
						{elegida?.thumb && (
							<img src={urlBusqueda(elegida.thumb)} alt="" className="size-full object-cover" />
						)}
					</div>
					{elegida && (
						<div className="text-xs">
							<div className="font-medium">{elegida.printedName ?? elegida.name}</div>
							<div className="text-muted-foreground">
								{elegida.setName} · {elegida.set.toUpperCase()} {elegida.collectorNumber} ·{" "}
								{elegida.lang}
							</div>
						</div>
					)}
				</div>
			</div>
			<DialogFooter>
				{/* Con una sola copia, «todas» y «solo esta» son lo mismo. */}
				{copias > 1 && (
					<Button
						size="sm"
						variant="outline"
						onClick={() => aplicar(true)}
						disabled={!elegida || aplicando}
					>
						<ReplaceAll />
						{t("Reemplazar las {n} copias", { n: copias })}
					</Button>
				)}
				<Button size="sm" onClick={() => aplicar(false)} disabled={!elegida || aplicando}>
					{aplicando ? <Loader2 className="animate-spin" /> : <Replace />}
					{t("Reemplazar solo esta")}
				</Button>
			</DialogFooter>
		</DialogContent>
	);
}
