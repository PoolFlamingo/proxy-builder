import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { OrdenCartas } from "@/bindings/OrdenCartas";
import { type Opcion, Selector } from "@/components/campos";
import { HojaMenu } from "@/components/hoja-menu";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { RaizScroll, useVisible } from "@/features/preview/visibilidad";
import { RejillaCartas } from "@/features/variants/rejilla-cartas";
import { useSugerencias } from "@/features/variants/sugerencias";
import { useFormato } from "@/lib/formato";
import { useBusqueda } from "@/stores/busqueda-store";
import { claveDeFiltros, cuantosFiltros, hayCriterio } from "./consulta";
import { FichaCartaDialog } from "./ficha-carta";
import { FiltrosPanel } from "./filtros-panel";

/** Lo que se espera al cambiar un filtro antes de buscar (se tocan varios seguidos). */
const ESPERA_FILTROS_MS = 350;
/** Nada de seleccionadas: tocar una carta abre su ficha. */
const NINGUNA = new Set<string>();

/**
 * **Buscar**: consultar cualquier carta sin tocar el deck. El campo admite un
 * nombre (en cualquier idioma) o la sintaxis de Scryfall; los filtros, en un
 * lateral con sitio o en una hoja si no. Abrir una carta enseña su ficha.
 */
export function BuscarScreen() {
	const { t, mensaje } = useLang();
	const { tamano } = useFormato();
	const conLateral = tamano === "escritorio";
	const filtros = useBusqueda((s) => s.filtros);
	const resultados = useBusqueda((s) => s.resultados);
	const total = useBusqueda((s) => s.total);
	const buscando = useBusqueda((s) => s.buscando);
	const hayMas = useBusqueda((s) => s.hayMas);
	const falloAlSeguir = useBusqueda((s) => s.falloAlSeguir);
	const poner = useBusqueda((s) => s.poner);
	const [hojaFiltros, setHojaFiltros] = useState(false);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
	const [verSugerencias, setVerSugerencias] = useState(false);
	const [sugerencias] = useSugerencias(filtros.exacto ? "" : filtros.texto);

	const lanzar = () => {
		setVerSugerencias(false);
		useBusqueda
			.getState()
			.buscar()
			.catch((e) => toast.error(mensaje(e)));
	};

	// Cambiar un filtro o el orden vuelve a buscar sola; al volver a la pantalla no.
	const clave = claveDeFiltros(filtros);
	const claveAntes = useRef(clave);
	const lanzarAhora = useRef(lanzar);
	lanzarAhora.current = lanzar;
	useEffect(() => {
		if (clave === claveAntes.current) return;
		claveAntes.current = clave;
		const id = setTimeout(() => lanzarAhora.current(), ESPERA_FILTROS_MS);
		return () => clearTimeout(id);
	}, [clave]);

	const ordenes: Opcion<OrdenCartas>[] = [
		{ valor: "nombre", etiqueta: t("Nombre") },
		{ valor: "fecha", etiqueta: t("Fecha de salida") },
		{ valor: "coste", etiqueta: t("Coste de maná") },
		{ valor: "rareza", etiqueta: t("Rareza") },
		{ valor: "fuerza", etiqueta: t("Fuerza") },
		{ valor: "eur", etiqueta: t("Precio (€)") },
		{ valor: "usd", etiqueta: t("Precio ($)") },
	];
	const nFiltros = cuantosFiltros(filtros);

	return (
		<div className="flex h-full min-h-0">
			{conLateral && (
				<aside className="w-64 shrink-0 border-r" data-tutorial="buscar-filtros">
					<ScrollArea className="h-full">
						<div className="p-2.5">
							<FiltrosPanel />
						</div>
					</ScrollArea>
				</aside>
			)}
			<div className="flex min-w-0 flex-1 flex-col">
				<div className="flex shrink-0 items-center gap-1.5 border-b p-2.5">
					<div className="relative min-w-0 flex-1" data-tutorial="buscar-campo">
						<Icono
							icon="lucide:search"
							className="pointer-events-none absolute top-1/2 left-2 size-4 -translate-y-1/2 text-muted-foreground"
						/>
						<Input
							value={filtros.texto}
							onChange={(e) => {
								poner({ texto: e.target.value, exacto: false });
								setVerSugerencias(true);
							}}
							onBlur={() => setVerSugerencias(false)}
							onKeyDown={(e) => {
								if (e.key === "Enter") lanzar();
								if (e.key === "Escape") setVerSugerencias(false);
							}}
							placeholder={t("Nombre de la carta…")}
							aria-label={t("Buscar")}
							enterKeyHint="search"
							className="h-8 pl-8 text-xs"
						/>
						{verSugerencias && sugerencias.length > 0 && (
							<div
								// Que tocar una sugerencia no le quite antes el foco al campo y la cierre.
								onMouseDown={(e) => e.preventDefault()}
								className="absolute inset-x-0 top-full z-20 mt-1 flex flex-col rounded-md border bg-popover p-1 shadow-lg"
							>
								{sugerencias.map((s) => (
									<button
										key={s}
										type="button"
										className="min-h-8 rounded-sm px-2 py-1 text-left text-xs hover:bg-accent"
										onClick={() => {
											poner({ texto: s, exacto: true });
											lanzar();
										}}
									>
										{s}
									</button>
								))}
							</div>
						)}
					</div>
					<Button size="sm" onClick={lanzar} disabled={buscando || !hayCriterio(filtros)}>
						{buscando ? (
							<Icono icon="lucide:loader-2" className="animate-spin" />
						) : (
							<Icono icon="lucide:search" />
						)}
						<span className="max-sm:sr-only">{t("Buscar")}</span>
					</Button>
					{!conLateral && (
						<Button
							size="sm"
							variant="outline"
							aria-label={t("Filtros")}
							data-tutorial="buscar-filtros"
							onClick={() => setHojaFiltros(true)}
						>
							<Icono icon="lucide:sliders-horizontal" />
							<span className="max-sm:sr-only">{t("Filtros")}</span>
							{nFiltros > 0 && (
								<span className="rounded-full bg-primary px-1.5 text-[10px] leading-4 font-semibold text-primary-foreground">
									{nFiltros}
								</span>
							)}
						</Button>
					)}
				</div>
				{resultados && resultados.length > 0 && (
					<div className="flex shrink-0 items-center gap-1.5 border-b px-2.5 py-1.5">
						<span className="mr-auto truncate text-xs text-muted-foreground">
							{t("{n} resultados", { n: total })}
						</span>
						<Selector
							valor={filtros.orden}
							opciones={ordenes}
							onChange={(orden) => poner({ orden })}
							ancho="w-36"
							ariaLabel={t("Ordenar por")}
						/>
						<WithTooltip content={filtros.descendente ? t("Descendente") : t("Ascendente")}>
							<Button
								size="icon-sm"
								variant="ghost"
								aria-label={filtros.descendente ? t("Descendente") : t("Ascendente")}
								onClick={() => poner({ descendente: !filtros.descendente })}
							>
								<Icono
									icon={
										filtros.descendente
											? "lucide:arrow-down-wide-narrow"
											: "lucide:arrow-up-narrow-wide"
									}
								/>
							</Button>
						</WithTooltip>
					</div>
				)}
				<ScrollArea
					className="min-h-0 flex-1"
					onViewportChange={setViewport}
					data-tutorial="buscar-resultados"
				>
					<RaizScroll.Provider value={viewport}>
						{resultados === null ? (
							<Aviso icono="lucide:search">
								{t(
									"Busca por nombre, en cualquier idioma, o con la sintaxis de Scryfall (t:dragon c:r).",
								)}
							</Aviso>
						) : resultados.length === 0 ? (
							buscando ? (
								<Aviso icono="lucide:loader-2" girando>
									{t("Buscando…")}
								</Aviso>
							) : (
								<Aviso icono="lucide:search-x">{t("Sin resultados.")}</Aviso>
							)
						) : (
							<>
								<RejillaCartas
									cartas={resultados}
									elegidas={NINGUNA}
									onElegir={(c) => useBusqueda.getState().abrirFicha(c.id)}
									columnas="grid-cols-[repeat(2,minmax(0,1fr))] sm:grid-cols-[repeat(auto-fill,minmax(140px,1fr))]"
								/>
								{hayMas &&
									(falloAlSeguir ? (
										<div className="flex justify-center p-2.5">
											<Button size="sm" variant="outline" onClick={() => void seguir(mensaje)}>
												{t("Reintentar")}
											</Button>
										</div>
									) : (
										// Al acercarse al final se piden más.
										<Mas alLlegar={() => void seguir(mensaje)} cuantas={resultados.length} />
									))}
							</>
						)}
					</RaizScroll.Provider>
				</ScrollArea>
			</div>
			{!conLateral && (
				<HojaMenu
					abierta={hojaFiltros}
					onCerrar={() => setHojaFiltros(false)}
					titulo={t("Filtros")}
					pie={<Button onClick={() => setHojaFiltros(false)}>{t("Ver los resultados")}</Button>}
				>
					<FiltrosPanel />
				</HojaMenu>
			)}
			<FichaCartaDialog />
		</div>
	);
}

async function seguir(mensaje: (e: unknown) => string) {
	try {
		await useBusqueda.getState().seguir();
	} catch (e) {
		toast.error(mensaje(e));
	}
}

/** El final de la lista: al verse, avisa (y otra vez cada vez que llegan más). */
function Mas({ alLlegar, cuantas }: { alLlegar: () => void; cuantas: number }) {
	const [el, setEl] = useState<HTMLDivElement | null>(null);
	const visible = useVisible(el);
	const avisar = useRef(alLlegar);
	avisar.current = alLlegar;
	useEffect(() => {
		if (visible) avisar.current();
	}, [visible, cuantas]);
	return (
		<div ref={setEl} className="flex justify-center p-2.5">
			<Icono icon="lucide:loader-2" className="size-4 animate-spin text-muted-foreground" />
		</div>
	);
}

function Aviso({
	icono,
	girando = false,
	children,
}: {
	icono: string;
	girando?: boolean;
	children: React.ReactNode;
}) {
	return (
		<div className="flex flex-col items-center gap-2.5 px-4 py-16 text-center text-xs text-muted-foreground">
			<Icono icon={icono} className={girando ? "size-6 animate-spin" : "size-6"} />
			<p className="max-w-80">{children}</p>
		</div>
	);
}
