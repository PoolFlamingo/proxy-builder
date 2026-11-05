import { type ReactNode, useEffect, useRef, useState } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import { Confirmar } from "@/components/confirmar";
import { Hoja } from "@/components/hoja";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { ManijaRedimensionar } from "@/components/manija-redimensionar";
import { Button } from "@/components/ui/button";
import { Dialog, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useAbrirPbdeck } from "@/features/library/abrir-pbdeck";
import { DialogosPasar } from "@/features/pasar/dialogos-pasar";
import { useEnlacesEntrantes } from "@/features/salas/enlaces-entrantes";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import { Tutorial, useTutorialInicial } from "@/features/tutorial/tutorial";
import { fijarAtrasPorDefecto, registrarAtras } from "@/lib/atras";
import { pendientes } from "@/lib/cierre";
import { useFormato } from "@/lib/formato";
import { engancharTeclado } from "@/lib/teclado";
import { useNombreDeck } from "@/lib/use-nombre-deck";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import { useEdicion } from "@/stores/edicion-store";
import { useUi, type Vista } from "@/stores/ui-store";
import { BotonActualizar, PuntoActualizacion, useAvisoAjustes } from "./actualizacion-automatica";
import { atrasPorDefecto, destinoDeAtras } from "./atras-shell";
import { BarraInferiorMovil, HojaMas, PestanasDeck, type SubpestanaDeck } from "./barra-movil";
import { ConfirmarCierre } from "./confirmar-cierre";
import { ControlesVentana } from "./controles-ventana";
import { useMesaAPantallaCompleta } from "./mesa-completa";
import { SaveIndicator } from "./save-indicator";
import { useScrollPorVista } from "./scroll-por-vista";

/**
 * El marco de la app, como fabricard: barra superior y, debajo, tres paneles
 * redimensionables — la lista de cartas a la izquierda, las páginas en el
 * centro y los ajustes de exportación a la derecha. Los anchos se recuerdan.
 *
 * Una sola interfaz que cambia de disposición según el formato
 * (`lib/formato.ts`): en **escritorio** es la de siempre; en **tableta**
 * (menos de 1024 px) el mismo encabezado más compacto, Deck con dos paneles y
 * Exportar en una hoja lateral; en **móvil**, una barra de arriba con el
 * nombre del deck, una de pestañas abajo (Deck, Diseñar, Biblioteca, Jugar y
 * Más) y Deck partido en tres pestañas (Lista, Hojas, Exportar). Con una
 * partida abierta en el móvil las dos barras se esconden y la mesa ocupa la
 * pantalla (`mesa-completa.ts`).
 */
export function AppShell({
	izquierda,
	centro,
	derecha,
	biblioteca,
	disenar,
	mesa,
}: {
	izquierda: ReactNode;
	centro: ReactNode;
	derecha: ReactNode;
	biblioteca: ReactNode;
	disenar: ReactNode;
	mesa: ReactNode;
}) {
	const lang = useLang();
	const { t } = lang;
	const { tamano, tactil, horizontal } = useFormato();
	const movil = tamano === "movil";
	// La disposición de tableta (encabezado compacto, dos paneles y Exportar en
	// una hoja) es para el dedo: una ventana de escritorio estrecha, con ratón,
	// sigue con los tres paneles de siempre.
	const tableta = tamano === "tableta" && tactil;
	const vista = useUi((s) => s.vista);
	const setVista = useUi((s) => s.setVista);
	const [ajustes, setAjustes] = useState(false);
	const [mas, setMas] = useState(false);
	const [exportar, setExportar] = useState(false);
	const [subpestana, setSubpestana] = useState<SubpestanaDeck>("lista");
	const avisoAjustes = useAvisoAjustes();
	const layout = useDefaultLayout({ id: "proxybuilder.paneles", storage: localStorage });
	const layoutTableta = useDefaultLayout({
		id: "proxybuilder.paneles-tableta",
		storage: localStorage,
	});
	const mesaCompleta = useMesaAPantallaCompleta();
	useTutorialInicial();
	const principal = useRef<HTMLElement>(null);
	useScrollPorVista(movil && vista === "deck" ? `deck:${subpestana}` : vista, principal);
	// Salir de «Diseñar» con la carta sin guardar pregunta, y se puede guardar
	// ahí mismo (como al cambiar de carta). En el móvil no se pregunta: se
	// guarda sola, como al irse a segundo plano (`lib/ciclo-de-vida.ts`).
	const [salirDeDisenar, setSalirDeDisenar] = useState<(() => void) | null>(null);
	const cambiarVista = (v: Vista) => {
		if (v === vista) return;
		if (vista === "disenar" && useEdicion.getState().pendiente) {
			if (movil) {
				void (async () => {
					const ok = (await useEdicion.getState().pendiente?.guardar()) ?? true;
					if (ok) setVista(v);
				})();
				return;
			}
			setSalirDeDisenar(() => () => setVista(v));
			return;
		}
		setVista(v);
	};

	useEnlacesEntrantes(() => cambiarVista("mesa"));
	// Un .pbdeck que otra app abre con esta (Android).
	useAbrirPbdeck();

	// El teclado de la pantalla no tapa lo que se escribe (`--alto-visible`).
	useEffect(() => (tactil ? engancharTeclado() : undefined), [tactil]);

	// El botón «atrás» de Android (`lib/atras.ts`). Antes de esto ya se ha
	// cerrado la capa de arriba (diálogo, hoja, menú) y lo que atiendan las
	// pantallas con sus paneles (la mesa). Aquí: desde Diseñar se vuelve a
	// Deck y, en cualquier otra, «pulsa otra vez para salir».
	const cambiar = useRef(cambiarVista);
	cambiar.current = cambiarVista;
	const textos = useRef(t);
	textos.current = t;
	useEffect(() => {
		if (!tactil) return;
		const soltar = registrarAtras(() => {
			const destino = destinoDeAtras(useUi.getState().vista);
			if (!destino) return false;
			cambiar.current(destino);
			return true;
		});
		fijarAtrasPorDefecto(
			atrasPorDefecto(
				() => textos.current("Pulsa otra vez para salir"),
				() => {
					const p = pendientes();
					return p.trabajo !== null || p.actualizando
						? textos.current("Hay un trabajo en marcha; espera a que termine.")
						: null;
				},
			),
		);
		return () => {
			soltar();
			fijarAtrasPorDefecto(null);
		};
	}, [tactil]);

	const pantalla = (
		<>
			<SettingsDialog open={ajustes} onOpenChange={setAjustes} />
			<Tutorial />
			<DialogosPasar />
			<ConfirmarCierre />
			<Confirmar
				abierto={!!salirDeDisenar}
				onCerrar={() => setSalirDeDisenar(null)}
				titulo={t("¿Descartar los cambios?")}
				descripcion={t("Hay cambios sin guardar en esta carta.")}
				accion={t("Descartar")}
				destructiva
				onConfirmar={() => salirDeDisenar?.()}
				accionAlternativa={{
					texto: t("Guardar y continuar"),
					onClick: async () => {
						const ok = (await useEdicion.getState().pendiente?.guardar()) ?? true;
						if (ok) salirDeDisenar?.();
						return ok;
					},
				}}
			/>
		</>
	);

	// Alto: en escritorio `svh`, como siempre. En táctil, el del `#root` (que ya
	// descuenta las zonas seguras) y, con el teclado abierto, hasta donde llega
	// lo que se ve (`--alto-visible`, `lib/teclado.ts`).
	const clasesRaiz = cn(
		"flex h-svh flex-col bg-background text-foreground",
		"[[data-tactil]_&]:h-[min(100%,calc(var(--alto-visible,100dvh)-env(safe-area-inset-top)))]",
	);

	if (movil) {
		return (
			// `overflow-clip`: nada de lo de dentro (una barra que no cabe, una
			// rejilla) puede ensanchar el documento y crear scroll horizontal de
			// toda la pantalla. Es `clip` y no `hidden` para que no sea un
			// contenedor de scroll al que el navegador pueda mover al enfocar algo.
			<div className={cn(clasesRaiz, "w-full overflow-clip", horizontal && "flex-row")}>
				<div className="flex min-h-0 min-w-0 flex-1 flex-col">
					{/* Diseñar no la lleva: su propia barra dice el deck y si está guardado. */}
					{!mesaCompleta && vista === "deck" && (
						// La barra de arriba del móvil (`BarraSuperior`): 3 rem, como las demás.
						<header className="flex h-12 min-w-0 shrink-0 items-center gap-1 border-b px-2">
							<Icono icon="lucide:layers" className="mx-1 size-5 shrink-0 text-primary" />
							<NombreDeck className="ml-0 h-10 w-auto min-w-0 flex-1 px-1.5 text-sm font-semibold dark:bg-transparent" />
							<span className="flex size-10 shrink-0 items-center justify-center">
								<SaveIndicator compacto />
							</span>
							{horizontal && (
								<PestanasDeck valor={subpestana} onCambiar={setSubpestana} enCabecera />
							)}
						</header>
					)}
					<main ref={principal} className="min-h-0 min-w-0 flex-1">
						{vista === "deck" ? (
							<div className="flex h-full min-h-0 flex-col">
								{!horizontal && <PestanasDeck valor={subpestana} onCambiar={setSubpestana} />}
								<div className="min-h-0 flex-1">
									{subpestana === "lista" ? izquierda : subpestana === "hojas" ? centro : derecha}
								</div>
							</div>
						) : vista === "disenar" ? (
							disenar
						) : vista === "mesa" ? (
							mesa
						) : (
							biblioteca
						)}
					</main>
				</div>
				{!mesaCompleta && (
					<BarraInferiorMovil
						vista={vista}
						onCambiar={cambiarVista}
						onMas={() => setMas(true)}
						masAbierto={mas}
						lado={horizontal}
					/>
				)}
				<HojaMas
					abierta={mas}
					onCerrar={() => setMas(false)}
					onAjustes={() => setAjustes(true)}
					vista={vista}
				/>
				{pantalla}
			</div>
		);
	}

	return (
		<div className={clasesRaiz}>
			<header
				className={cn(
					"flex h-12 shrink-0 items-center gap-2.5 border-b px-3 [[data-tactil]_&]:h-14",
					tableta && "gap-1.5 px-2",
				)}
			>
				<div className="flex items-center gap-2">
					<Icono icon="lucide:layers" className="size-5 text-primary" />
					<div className={cn("leading-tight", tableta && "hidden")}>
						<div className="text-sm font-semibold">Proxy Builder</div>
						<div className="hidden text-[10px] text-muted-foreground lg:block">
							{t("Scryfall · listo para imprimir")}
						</div>
					</div>
				</div>
				<ToggleGroup
					type="single"
					size="sm"
					data-pestanas=""
					data-tutorial="pestanas"
					className={cn("ml-4 shrink-0", tableta && "ml-1")}
					value={vista}
					onValueChange={(v) =>
						(v === "deck" || v === "disenar" || v === "biblioteca" || v === "mesa") &&
						cambiarVista(v)
					}
				>
					<ToggleGroupItem
						value="deck"
						className={cn("gap-1.5 px-3 text-xs", tableta && "px-2")}
						aria-label={t("Deck")}
					>
						<Icono icon="lucide:layers" className="size-3.5" />
						<span className="max-[860px]:hidden">{t("Deck")}</span>
					</ToggleGroupItem>
					<ToggleGroupItem
						value="disenar"
						className={cn("gap-1.5 px-3 text-xs", tableta && "px-2")}
						aria-label={t("Diseñar")}
					>
						<Icono icon="lucide:brush" className="size-3.5" />
						<span className="max-[860px]:hidden">{t("Diseñar")}</span>
					</ToggleGroupItem>
					<ToggleGroupItem
						value="biblioteca"
						className={cn("gap-1.5 px-3 text-xs", tableta && "px-2")}
						aria-label={t("Biblioteca")}
					>
						<Icono icon="lucide:book-open" className="size-3.5" />
						<span className="max-[860px]:hidden">{t("Biblioteca")}</span>
					</ToggleGroupItem>
					<ToggleGroupItem
						value="mesa"
						className={cn("group gap-1.5 px-3 text-xs", tableta && "px-2")}
						aria-label={t("Jugar")}
					>
						<Icono icon="game-icons:crossed-swords" className="size-3.5" />
						<span className="max-[860px]:hidden">{t("Jugar")}</span>
						<span className="rounded-full bg-primary/15 px-1.5 text-[9px] font-semibold uppercase leading-4 text-primary group-data-[state=on]:bg-accent-foreground/20 group-data-[state=on]:text-accent-foreground max-[860px]:hidden">
							{t("Beta")}
						</span>
					</ToggleGroupItem>
				</ToggleGroup>
				{vista !== "biblioteca" && vista !== "mesa" && <NombreDeck />}
				<div className={cn("ml-auto flex items-center gap-2.5", tableta && "gap-1.5")}>
					<SaveIndicator />
					<BotonActualizar />
					{tableta && vista === "deck" && (
						<Button
							size="sm"
							variant="outline"
							data-tutorial="exportar-boton"
							onClick={() => setExportar(true)}
						>
							<Icono icon="lucide:printer" className="size-4" />
							{t("Exportar")}
						</Button>
					)}
					<WithTooltip content={t("Tutorial de esta pantalla")}>
						<Button
							size="icon-sm"
							variant="ghost"
							data-tutorial="ayuda"
							onClick={() => useUi.getState().abrirTutorial(vista)}
							aria-label={t("Tutorial de esta pantalla")}
						>
							<Icono icon="lucide:circle-help" className="size-4" />
						</Button>
					</WithTooltip>
					{/* Estrecha la ventana, solo el icono: el tooltip dice qué es. */}
					<WithTooltip content={avisoAjustes ?? t("Ajustes")}>
						<Button
							size="sm"
							variant="ghost"
							data-tutorial="ajustes"
							onClick={() => setAjustes(true)}
							aria-label={avisoAjustes ? `${t("Ajustes")}: ${avisoAjustes}` : t("Ajustes")}
							className="relative"
						>
							<Icono icon="lucide:settings" className="size-4" />
							<span className="hidden lg:inline">{t("Ajustes")}</span>
							<PuntoActualizacion />
						</Button>
					</WithTooltip>
					{/* Solo a pantalla completa: el sistema esconde sus botones. */}
					<ControlesVentana />
				</div>
			</header>

			<main ref={principal} className="min-h-0 flex-1">
				{vista === "deck" ? (
					tableta ? (
						<ResizablePanelGroup
							orientation="horizontal"
							defaultLayout={layoutTableta.defaultLayout}
							onLayoutChanged={layoutTableta.onLayoutChanged}
						>
							<ResizablePanel id="izquierda" defaultSize="38" minSize={260}>
								{izquierda}
							</ResizablePanel>
							<ManijaRedimensionar withHandle />
							<ResizablePanel id="centro" minSize={360}>
								{centro}
							</ResizablePanel>
						</ResizablePanelGroup>
					) : (
						<ResizablePanelGroup
							orientation="horizontal"
							defaultLayout={layout.defaultLayout}
							onLayoutChanged={layout.onLayoutChanged}
						>
							<ResizablePanel id="izquierda" defaultSize="22" minSize={260}>
								{izquierda}
							</ResizablePanel>
							<ManijaRedimensionar withHandle />
							<ResizablePanel id="centro" minSize={360}>
								{centro}
							</ResizablePanel>
							<ManijaRedimensionar withHandle />
							<ResizablePanel id="derecha" defaultSize="22" minSize={260}>
								{derecha}
							</ResizablePanel>
						</ResizablePanelGroup>
					)
				) : vista === "disenar" ? (
					disenar
				) : vista === "mesa" ? (
					mesa
				) : (
					biblioteca
				)}
			</main>

			{/* Tableta: Exportar sale de un lado, con su botón de la barra. */}
			{tableta && (
				<Dialog open={exportar && vista === "deck"} onOpenChange={setExportar}>
					<Hoja
						lado="derecha"
						cambiosSinGuardar={false}
						aria-describedby={undefined}
						className="gap-0 p-0 pt-[env(safe-area-inset-top)]"
					>
						<DialogTitle className="sr-only">{t("Exportar")}</DialogTitle>
						<div className="min-h-0 flex-1">{derecha}</div>
					</Hoja>
				</Dialog>
			)}
			{pantalla}
		</div>
	);
}

/** El nombre del deck abierto, editable en la barra (se guarda con el deck). */
function NombreDeck({ className }: { className?: string }) {
	const { t } = useLang();
	const guardado = useDeck((s) => s.deck?.name ?? "");
	// El de fábrica, en el idioma de ahora (`lib/nombre-deck.ts`): si no se
	// toca, lo guardado no cambia.
	const nombre = useNombreDeck()(guardado);
	const [texto, setTexto] = useState(nombre);
	useEffect(() => setTexto(nombre), [nombre]);
	const confirmar = () => {
		const limpio = texto.trim();
		if (!limpio) return setTexto(nombre);
		if (limpio !== nombre) useDeck.getState().setSinHistorial({ name: limpio });
	};
	// Al irse a segundo plano (Android) el campo no pierde el foco y su texto se
	// perdería: se confirma antes de que `lib/ciclo-de-vida.ts` guarde el deck
	// (con `capture`, el primero en oír el `visibilitychange`).
	const alIrse = useRef(confirmar);
	alIrse.current = confirmar;
	useEffect(() => {
		const f = () => {
			if (document.visibilityState === "hidden") alIrse.current();
		};
		document.addEventListener("visibilitychange", f, { capture: true });
		return () => document.removeEventListener("visibilitychange", f, { capture: true });
	}, []);
	return (
		<Input
			value={texto}
			onChange={(e) => setTexto(e.target.value)}
			onBlur={confirmar}
			onKeyDown={(e) => {
				if (e.key === "Enter") (e.target as HTMLInputElement).blur();
				if (e.key === "Escape") setTexto(nombre);
			}}
			aria-label={t("Nombre del deck")}
			data-tutorial="nombre-deck"
			className={cn(
				"ml-2 h-7 w-64 min-w-24 shrink border-transparent bg-transparent text-sm font-medium shadow-none hover:border-input focus-visible:border-input",
				className,
			)}
		/>
	);
}
