import { FolderOpen, Loader2, Package, RefreshCw, Trash2, Type } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { LetraInfo } from "@/bindings/LetraInfo";
import type { PackInfo } from "@/bindings/PackInfo";
import type { SimbolosInfo } from "@/bindings/SimbolosInfo";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { abrirParaImportar } from "@/lib/dialogos";
import { call } from "@/lib/ipc";
import { useUi } from "@/stores/ui-store";
import { rehacerMiniaturas } from "./renumerar";
import { ESTILOS } from "./textos";

/**
 * **Letras, marcos y símbolos**: las letras y los packs de marcos importados
 * (las de la casa siempre están) y el estado de los símbolos de maná.
 */
export function RecursosDialog() {
	const abierto = useUi((s) => s.dialogo?.tipo === "recursosDiseno");
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{abierto && <Contenido />}
		</Dialog>
	);
}

function kb(bytes: bigint | number): string {
	return `${Math.max(1, Math.round(Number(bytes) / 1024))} KB`;
}

function Contenido() {
	const { t, lang, mensaje } = useLang();
	const [letras, setLetras] = useState<LetraInfo[] | null>(null);
	const [packs, setPacks] = useState<PackInfo[] | null>(null);
	const [simbolos, setSimbolos] = useState<SimbolosInfo | null>(null);
	const [ocupado, setOcupado] = useState<string | null>(null);
	const tocado = useRef(false);

	const recargar = useCallback(() => {
		call("fonts_list", {})
			.then(setLetras)
			.catch((e) => toast.error(mensaje(e)));
		call("packs_list", {})
			.then(setPacks)
			.catch((e) => toast.error(mensaje(e)));
	}, [mensaje]);

	useEffect(() => {
		recargar();
		call("symbols_status", { refresh: false })
			.then(setSimbolos)
			.catch(() => {});
		// Al cerrar, si algo cambió, se rehacen las miniaturas viejas.
		return () => {
			if (tocado.current) void rehacerMiniaturas();
		};
	}, [recargar]);

	/** Corre una acción marcando qué botón está ocupado. */
	const con = async (clave: string, f: () => Promise<void>) => {
		setOcupado(clave);
		try {
			await f();
			tocado.current = true;
			recargar();
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(null);
		}
	};

	const importarLetras = () =>
		con("letra", async () => {
			const rutas = await abrirParaImportar({
				multiple: true,
				filters: [{ name: t("Letras"), extensions: ["ttf", "otf"] }],
			});
			if (!rutas) return;
			for (const path of rutas) await call("font_import", { path });
			useUi.getState().anunciar(t("{n} letras importadas", { n: rutas.length }));
		});

	const importarPack = (carpeta: boolean) =>
		con("pack", async () => {
			const rutas = await abrirParaImportar(
				carpeta
					? { directory: true, multiple: false }
					: { multiple: false, filters: [{ name: t("Pack de marcos"), extensions: ["zip"] }] },
			);
			const ruta = rutas?.[0];
			if (!ruta) return;
			const p = await call("pack_import", { path: ruta });
			useUi.getState().anunciar(t("Pack «{nombre}» importado", { nombre: p.nombre }));
		});

	const refrescarSimbolos = async () => {
		setOcupado("simbolos");
		try {
			setSimbolos(await call("symbols_status", { refresh: true }));
			tocado.current = true;
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(null);
		}
	};

	const fecha = simbolos?.fecha
		? new Date(simbolos.fecha).toLocaleDateString(lang === "en" ? "en-GB" : "es-ES", {
				day: "numeric",
				month: "long",
				year: "numeric",
			})
		: null;

	return (
		<DialogContent className="flex h-[min(80vh,620px)] flex-col sm:max-w-2xl">
			<DialogHeader>
				<DialogTitle>{t("Letras, marcos y símbolos")}</DialogTitle>
				<DialogDescription className="text-xs">
					{t(
						"Lo que usa el editor para pintar las cartas. Lo de la casa siempre está; aquí se añade lo tuyo.",
					)}
				</DialogDescription>
			</DialogHeader>
			<Tabs defaultValue="letras" className="flex min-h-0 flex-1 flex-col">
				<TabsList>
					<TabsTrigger value="letras" className="text-xs">
						{t("Letras")}
					</TabsTrigger>
					<TabsTrigger value="packs" className="text-xs">
						{t("Packs de marcos")}
					</TabsTrigger>
					<TabsTrigger value="simbolos" className="text-xs">
						{t("Símbolos")}
					</TabsTrigger>
				</TabsList>

				<TabsContent value="letras" className="flex min-h-0 flex-1 flex-col gap-2.5">
					<div className="flex items-center gap-2.5">
						<p className="flex-1 text-xs text-muted-foreground">
							{t(
								"De la casa: Spectral (título y reglas) y Montserrat (pie). Importa .ttf u .otf para usarlas en una colección.",
							)}
						</p>
						<Button
							size="sm"
							variant="outline"
							onClick={importarLetras}
							disabled={ocupado !== null}
						>
							{ocupado === "letra" ? <Loader2 className="animate-spin" /> : <Type />}
							{t("Importar letras…")}
						</Button>
					</div>
					<ScrollArea className="min-h-0 flex-1 rounded-md border">
						<ul className="flex flex-col divide-y">
							{letras === null && (
								<Loader2 className="m-3 size-4 animate-spin text-muted-foreground" />
							)}
							{letras?.length === 0 && (
								<li className="p-3 text-xs text-muted-foreground">
									{t("No has importado ninguna letra.")}
								</li>
							)}
							{letras?.map((l) => (
								<li key={l.id} className="flex items-center gap-2.5 px-3 py-2">
									<div className="min-w-0 flex-1">
										<div className="truncate text-xs font-medium">{l.familias.join(", ")}</div>
										<div className="font-mono text-[10px] text-muted-foreground">
											{l.id.slice(0, 12)} · {kb(l.bytes)}
										</div>
									</div>
									<WithTooltip content={t("Las cartas que la usen pasarán a la letra del pack.")}>
										<Button
											size="sm"
											variant="ghost"
											aria-label={t("Quitar letra")}
											disabled={ocupado !== null}
											onClick={() =>
												con(l.id, () => call("font_remove", { id: l.id }).then(() => {}))
											}
										>
											<Trash2 />
										</Button>
									</WithTooltip>
								</li>
							))}
						</ul>
					</ScrollArea>
				</TabsContent>

				<TabsContent value="packs" className="flex min-h-0 flex-1 flex-col gap-2.5">
					<div className="flex items-center gap-1.5">
						<p className="flex-1 text-xs text-muted-foreground">
							{t("Un pack es una carpeta (o un .zip) con su pack.json y sus capas en SVG.")}
						</p>
						<Button
							size="sm"
							variant="outline"
							onClick={() => importarPack(true)}
							disabled={ocupado !== null}
						>
							{ocupado === "pack" ? <Loader2 className="animate-spin" /> : <FolderOpen />}
							{t("Carpeta…")}
						</Button>
						<Button
							size="sm"
							variant="outline"
							onClick={() => importarPack(false)}
							disabled={ocupado !== null}
						>
							<Package />
							{t(".zip…")}
						</Button>
					</div>
					<ScrollArea className="min-h-0 flex-1 rounded-md border">
						<ul className="flex flex-col divide-y">
							{packs === null && (
								<Loader2 className="m-3 size-4 animate-spin text-muted-foreground" />
							)}
							{packs?.map((p) => (
								<li key={p.id} className="flex items-center gap-2.5 px-3 py-2">
									<div className="min-w-0 flex-1">
										<div className="truncate text-xs font-medium">
											{p.importado ? p.nombre : t("Proxy Builder (de la casa)")}
										</div>
										<div className="truncate text-[10px] text-muted-foreground">
											{t("Estilos: {estilos}", {
												estilos: p.estilos
													.map((e) => {
														const conocido = ESTILOS.find((x) => x.valor === e);
														return conocido ? t(conocido.etiqueta) : e;
													})
													.join(", "),
											})}
										</div>
									</div>
									{p.importado && (
										<WithTooltip content={t("Las cartas que lo usen pasarán al pack de la casa.")}>
											<Button
												size="sm"
												variant="ghost"
												aria-label={t("Quitar pack")}
												disabled={ocupado !== null}
												onClick={() =>
													con(p.id, () => call("pack_remove", { id: p.id }).then(() => {}))
												}
											>
												<Trash2 />
											</Button>
										</WithTooltip>
									)}
								</li>
							))}
						</ul>
					</ScrollArea>
				</TabsContent>

				<TabsContent value="simbolos" className="flex flex-col gap-2.5">
					<p className="text-xs text-muted-foreground">
						{t(
							"Los símbolos de maná y de reglas vienen de Scryfall: se descargan la primera vez y se renuevan cada mes.",
						)}
					</p>
					<div className="flex items-center gap-2.5 rounded-md border p-3">
						<div className="flex-1 text-xs">
							{simbolos === null ? (
								<Loader2 className="size-4 animate-spin text-muted-foreground" />
							) : simbolos.cargados === 0 ? (
								t("No hay símbolos: hace falta conexión para descargarlos.")
							) : (
								<>
									<div className="font-medium">{t("{n} símbolos", { n: simbolos.cargados })}</div>
									{fecha && (
										<div className="text-[11px] text-muted-foreground">
											{t("Descargados el {fecha}", { fecha })}
										</div>
									)}
								</>
							)}
						</div>
						<Button
							size="sm"
							variant="outline"
							onClick={refrescarSimbolos}
							disabled={ocupado !== null}
						>
							{ocupado === "simbolos" ? <Loader2 className="animate-spin" /> : <RefreshCw />}
							{t("Descargar de nuevo")}
						</Button>
					</div>
				</TabsContent>
			</Tabs>
		</DialogContent>
	);
}
