import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { DeckOptimizable } from "@/bindings/DeckOptimizable";
import { Confirmar } from "@/components/confirmar";
import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	Dialog,
	DialogCuerpo,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { megas } from "@/features/export/acciones";
import { useEstadoParaCerrar } from "@/lib/dialogos-abiertos";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { type Devolver, prestarDeck, useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { useBiblioteca } from "./store";

/**
 * **Liberar espacio**: borrar las imágenes que no usa nadie y optimizar las de
 * los decks (Rust: `optimizar.rs`), que sustituye los originales de este equipo
 * y no se puede deshacer: se eligen los decks y se confirma.
 */
export function DialogoLiberar({ abierto, onCerrar }: { abierto: boolean; onCerrar: () => void }) {
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && onCerrar()}>
			{abierto && <Contenido onCerrar={onCerrar} />}
		</Dialog>
	);
}

function Contenido({ onCerrar }: { onCerrar: () => void }) {
	const { t, mensaje } = useLang();
	const enDisenar = useUi((s) => s.vista === "disenar");
	const [decks, setDecks] = useState<DeckOptimizable[] | null>(null);
	const [ids, setIds] = useState<string[]>([]);
	const [libera, setLibera] = useState(0);
	const [borrando, setBorrando] = useState(false);
	const [confirmar, setConfirmar] = useState(false);
	const [avance, setAvance] = useState<number | null>(null);
	const trabajo = useRef<number | null>(null);
	const enMarcha = avance !== null;
	useEstadoParaCerrar({ enCurso: enMarcha, sinGuardar: false });

	const leer = () =>
		call("optimizar_decks", {})
			.then((l) => {
				setDecks(l);
				setIds(l.map((d) => d.id));
			})
			.catch((e) => {
				setDecks([]);
				toast.error(mensaje(e));
			});
	// Solo al abrir.
	useEffect(() => {
		void leer();
	}, []);

	useEffect(() => {
		let vivo = true;
		if (ids.length === 0) {
			setLibera(0);
			return;
		}
		call("optimizar_estimar", { ids })
			.then((e) => vivo && setLibera(e.libera))
			.catch(() => {});
		return () => {
			vivo = false;
		};
	}, [ids]);

	const borrarSinUsar = async () => {
		setBorrando(true);
		try {
			const r = await call("blob_gc", {});
			toast.success(
				r.removed > 0
					? t("{n} imágenes sin usar borradas ({mb})", { n: r.removed, mb: megas(r.freedBytes) })
					: t("No había imágenes sin usar (se guardan 24 h por si acaso)."),
			);
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setBorrando(false);
		}
	};

	const optimizar = async () => {
		setAvance(0);
		// Si está el deck abierto, Rust lo reescribe: lo pendiente se guarda antes
		// y se vuelve a cargar sin historial (deshacer apuntaría a los originales).
		const abierto = useDeck.getState().deck?.id;
		let devolver: Devolver | null = null;
		try {
			if (abierto && ids.includes(abierto)) {
				devolver = await prestarDeck(abierto, { conservar: false });
			}
			const j = await lanzarTrabajo(
				"optimizar://progress",
				"optimizar://done",
				() => call("optimizar_lanzar", { ids }),
				(p) => setAvance(p.total > 0 ? Math.round((p.hechas / p.total) * 100) : 0),
			);
			trabajo.current = j.jobId;
			const fin = await j.fin;
			if (fin.error) toast.error(mensaje(fin.error));
			else if (fin.cancelado) toast.info(t("Optimización cancelada: no ha cambiado nada."));
			else toast.success(t("Imágenes optimizadas: {mb} liberados", { mb: megas(fin.libera) }));
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			trabajo.current = null;
			await devolver?.();
			setAvance(null);
			void useBiblioteca.getState().refrescar();
			void leer();
		}
	};

	const alternar = (id: string) =>
		setIds((v) => (v.includes(id) ? v.filter((x) => x !== id) : [...v, id]));

	return (
		<DialogoAdaptable className="text-xs sm:max-w-lg" cambiosSinGuardar={false} ocupado={enMarcha}>
			<DialogHeader>
				<DialogTitle>{t("Liberar espacio")}</DialogTitle>
				<DialogDescription className="text-xs">
					{t("Lo que ocupan las imágenes de tu biblioteca en este equipo.")}
				</DialogDescription>
			</DialogHeader>
			<DialogCuerpo>
				<div className="flex flex-col gap-3 p-1">
					<div className="flex flex-wrap items-center justify-between gap-2.5 rounded-lg border p-2.5">
						<span className="text-muted-foreground">
							{t("Imágenes que ya no usa ningún deck.")}
						</span>
						<Button
							size="xs"
							variant="outline"
							disabled={borrando || enMarcha}
							onClick={() => void borrarSinUsar()}
						>
							<Icono icon="lucide:trash-2" />
							{t("Borrar")}
						</Button>
					</div>

					<div className="flex flex-col gap-2 rounded-lg border p-2.5">
						<span className="font-medium">{t("Optimizar las imágenes de los decks")}</span>
						<p className="text-muted-foreground">
							{t(
								"Pasa los PNG sin transparencia a JPEG con las mismas medidas: ocupan mucho menos y al imprimir no se nota.",
							)}
						</p>
						{decks === null ? (
							<Icono icon="lucide:loader-2" className="size-4 animate-spin text-muted-foreground" />
						) : decks.length === 0 ? (
							<p className="text-muted-foreground">
								{t("Las imágenes de tus decks ya están optimizadas.")}
							</p>
						) : (
							<>
								<ScrollArea className="rounded-lg border" viewportClassName="max-h-44">
									<ul className="flex flex-col gap-0.5 p-1">
										{decks.map((d) => (
											<li key={d.id}>
												<label className="flex min-h-7 cursor-pointer items-center gap-2 rounded-md px-1.5 py-1 hover:bg-accent">
													<Checkbox
														checked={ids.includes(d.id)}
														disabled={enMarcha}
														onCheckedChange={() => alternar(d.id)}
													/>
													<span className="min-w-0 flex-1 truncate">{d.nombre}</span>
													<span className="shrink-0 text-muted-foreground">
														{t("≈ {mb} menos", { mb: megas(d.ahorro) })}
													</span>
												</label>
											</li>
										))}
									</ul>
								</ScrollArea>
								<div className="flex items-start gap-1.5 rounded-lg border border-amber-500/40 bg-amber-500/10 p-2">
									<Icono
										icon="lucide:triangle-alert"
										className="mt-0.5 size-3.5 shrink-0 text-amber-500"
									/>
									<span>
										{t(
											"No se puede deshacer: las imágenes originales se borran de este equipo. Si quieres conservarlas, guarda antes el deck en «Calidad original» desde «Compartir».",
										)}
									</span>
								</div>
								{enDisenar && (
									<p className="text-amber-500">
										{t("Sal de «Diseñar» para optimizar las imágenes.")}
									</p>
								)}
								{enMarcha ? (
									<div className="flex items-center gap-2">
										<Progress value={avance} className="min-w-0 flex-1" />
										<Button
											size="xs"
											variant="outline"
											disabled={trabajo.current === null}
											onClick={() => {
												if (trabajo.current !== null)
													void call("job_cancel", { jobId: trabajo.current });
											}}
										>
											{t("Cancelar")}
										</Button>
									</div>
								) : (
									<div className="flex flex-wrap items-center justify-between gap-2.5">
										<span className="text-muted-foreground">
											{t("Se liberarán unos {mb}", { mb: megas(libera) })}
										</span>
										<Button
											size="xs"
											disabled={ids.length === 0 || enDisenar}
											onClick={() => setConfirmar(true)}
										>
											<Icono icon="lucide:minimize-2" />
											{t("Optimizar")}
										</Button>
									</div>
								)}
							</>
						)}
					</div>
				</div>
			</DialogCuerpo>
			<DialogFooter>
				<Button size="sm" variant="outline" onClick={onCerrar} disabled={enMarcha}>
					{t("Cerrar")}
				</Button>
			</DialogFooter>
			<Confirmar
				abierto={confirmar}
				onCerrar={() => setConfirmar(false)}
				titulo={t("¿Optimizar las imágenes?")}
				descripcion={t(
					"Se comprimirán las imágenes de los decks elegidos y se borrarán las originales de este equipo. No se puede deshacer.",
				)}
				accion={t("Optimizar y borrar los originales")}
				destructiva
				onConfirmar={() => void optimizar()}
			/>
		</DialogoAdaptable>
	);
}
