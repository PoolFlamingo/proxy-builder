import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { CardSummary } from "@/bindings/CardSummary";
import type { OrigenTapete } from "@/bindings/OrigenTapete";
import type { Tapete } from "@/bindings/Tapete";
import { FilaDeslizador } from "@/components/campos";
import { Icono } from "@/components/icono";
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
import { abrirParaImportar } from "@/lib/dialogos";
import { urlArte, urlBusqueda } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { usePantalla } from "./pantalla";

/**
 * **El tapete de un deck**: una imagen del equipo o la ilustración de una
 * carta de Scryfall, con un velo para que se lean las cartas. Gratis y del
 * usuario (va al almacén de imágenes). Sin tapete propio, sale el general.
 */
export function TapeteDialog({
	abierto,
	onCerrar,
	deckId,
}: {
	abierto: boolean;
	onCerrar: () => void;
	deckId: string | null;
}) {
	const { t, mensaje } = useLang();
	const [general, setGeneral] = useState(false);
	const deck = general ? null : deckId;
	const [tapete, setTapete] = useState<Tapete | null>(null);
	const [velo, setVelo] = useState(40);
	const [desenfoque, setDesenfoque] = useState(0);
	const [q, setQ] = useState("");
	const [resultados, setResultados] = useState<CardSummary[]>([]);
	const [ocupado, setOcupado] = useState(false);

	useEffect(() => {
		if (!abierto) return;
		call("mesa_tapete_get", { deck })
			.then((tp) => {
				setTapete(tp);
				if (tp) {
					setVelo(tp.velo);
					setDesenfoque(tp.desenfoque);
				}
			})
			.catch(() => setTapete(null));
	}, [abierto, deck]);

	const poner = async (origen: OrigenTapete | null) => {
		setOcupado(true);
		try {
			setTapete(await call("mesa_tapete_set", { deck, origen, velo, desenfoque }));
			usePantalla.getState().avisarTapete();
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(false);
		}
	};

	const deFichero = async () => {
		const rutas = await abrirParaImportar({
			multiple: false,
			filters: [{ name: t("Imágenes"), extensions: ["png", "jpg", "jpeg", "webp"] }],
		});
		if (rutas?.[0]) await poner({ tipo: "fichero", ruta: rutas[0] });
	};

	const buscar = async () => {
		if (!q.trim()) return;
		setOcupado(true);
		try {
			setResultados(await call("card_search", { q: q.trim(), lang: null, highresOnly: false }));
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(false);
		}
	};

	const ajustes = (v: number, d: number) => {
		if (tapete)
			void call("mesa_tapete_ajustes", { deck, velo: v, desenfoque: d })
				.then(() => usePantalla.getState().avisarTapete())
				.catch(() => {});
	};

	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && onCerrar()}>
			<DialogContent className="sm:max-w-2xl" cambiosSinGuardar={false} ocupado={ocupado}>
				<DialogHeader>
					<DialogTitle>{t("Tapete")}</DialogTitle>
					<DialogDescription>
						{t(
							"El fondo de tu lado de la mesa. Los demás jugadores lo verán tal cual en las partidas en línea.",
						)}
					</DialogDescription>
				</DialogHeader>
				<div className="flex flex-col gap-2.5">
					<label className="flex items-center gap-2.5 text-xs">
						<Switch checked={general} onCheckedChange={setGeneral} />
						{t("El general (para los decks sin tapete propio)")}
					</label>
					<div className="relative aspect-[16/7] overflow-hidden rounded-lg border bg-muted">
						{tapete ? (
							<>
								<img
									src={urlArte(tapete.blob, 1000)}
									alt=""
									className="size-full object-cover"
									style={{ filter: `blur(${desenfoque / 2}px)` }}
								/>
								<div className="absolute inset-0 bg-black" style={{ opacity: velo / 100 }} />
							</>
						) : (
							<div className="flex size-full items-center justify-center text-xs text-muted-foreground">
								{t("Sin tapete: el fondo del tema.")}
							</div>
						)}
					</div>
					<div className="grid grid-cols-2 gap-2.5">
						<FilaDeslizador
							etiqueta={t("Velo")}
							valor={velo}
							onChange={setVelo}
							onSoltar={(v) => ajustes(v, desenfoque)}
							min={0}
							max={90}
							sufijo=" %"
						/>
						<FilaDeslizador
							etiqueta={t("Desenfoque")}
							valor={desenfoque}
							onChange={setDesenfoque}
							onSoltar={(d) => ajustes(velo, d)}
							min={0}
							max={20}
							sufijo=" px"
						/>
					</div>
					<div className="flex gap-2.5">
						<Button size="sm" variant="outline" onClick={() => void deFichero()} disabled={ocupado}>
							<Icono icon="lucide:folder-open" className="size-3.5" />
							{t("Imagen del equipo…")}
						</Button>
						<Button
							size="sm"
							variant="ghost"
							onClick={() => void poner(null)}
							disabled={ocupado || !tapete}
						>
							<Icono icon="lucide:trash-2" className="size-3.5" />
							{t("Quitar")}
						</Button>
					</div>
					<div className="flex gap-2.5" data-filtro="">
						<Input
							value={q}
							onChange={(e) => setQ(e.target.value)}
							onKeyDown={(e) => e.key === "Enter" && void buscar()}
							placeholder={t("Ilustración de una carta de Scryfall…")}
							className="h-7 text-xs"
						/>
						<Button size="sm" onClick={() => void buscar()} disabled={ocupado}>
							{ocupado ? (
								<Icono icon="lucide:loader-2" className="size-3.5 animate-spin" />
							) : (
								<Icono icon="lucide:search" className="size-3.5" />
							)}
							{t("Buscar")}
						</Button>
					</div>
					{resultados.length > 0 && (
						<ScrollArea className="h-56">
							<div className="grid grid-cols-6 gap-2.5 p-1">
								{resultados.map((r) => (
									<button
										key={r.id}
										type="button"
										disabled={ocupado}
										onClick={() => void poner({ tipo: "scryfall", scryfallId: r.id })}
										className="overflow-hidden rounded-md border hover:ring-2 hover:ring-primary"
									>
										{r.thumb && <img src={urlBusqueda(r.thumb)} alt={r.name} loading="lazy" />}
									</button>
								))}
							</div>
						</ScrollArea>
					)}
				</div>
			</DialogContent>
		</Dialog>
	);
}
