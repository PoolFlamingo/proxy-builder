import { BookOpen, Brush, CircleHelp, Layers, Settings, Swords } from "lucide-react";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useEnlacesEntrantes } from "@/features/salas/enlaces-entrantes";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import { Tutorial, useTutorialInicial } from "@/features/tutorial/tutorial";
import { useDeck } from "@/stores/deck-store";
import { useEdicion } from "@/stores/edicion-store";
import { useUi, type Vista } from "@/stores/ui-store";
import { BotonActualizar, PuntoActualizacion, useAvisoAjustes } from "./actualizacion-automatica";
import { ConfirmarCierre } from "./confirmar-cierre";
import { SaveIndicator } from "./save-indicator";
import { useScrollPorVista } from "./scroll-por-vista";

/**
 * El marco de la app, como fabricard: barra superior y, debajo, tres paneles
 * redimensionables — la lista de cartas a la izquierda, las páginas en el
 * centro y los ajustes de exportación a la derecha. Los anchos se recuerdan.
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
	const { t } = useLang();
	const vista = useUi((s) => s.vista);
	const setVista = useUi((s) => s.setVista);
	const [ajustes, setAjustes] = useState(false);
	const avisoAjustes = useAvisoAjustes();
	const layout = useDefaultLayout({ id: "proxybuilder.paneles", storage: localStorage });
	useTutorialInicial();
	const principal = useRef<HTMLElement>(null);
	useScrollPorVista(vista, principal);
	// Salir de «Diseñar» con la carta sin guardar pregunta, y se puede guardar
	// ahí mismo (como al cambiar de carta).
	const [salirDeDisenar, setSalirDeDisenar] = useState<(() => void) | null>(null);
	const cambiarVista = (v: Vista) => {
		if (v === vista) return;
		if (vista === "disenar" && useEdicion.getState().pendiente) {
			setSalirDeDisenar(() => () => setVista(v));
			return;
		}
		setVista(v);
	};

	useEnlacesEntrantes(() => cambiarVista("mesa"));

	return (
		<div className="flex h-svh flex-col bg-background text-foreground">
			<header className="flex h-12 shrink-0 items-center gap-2.5 border-b px-3">
				<div className="flex items-center gap-2">
					<Layers className="size-5 text-primary" />
					<div className="leading-tight">
						<div className="text-sm font-semibold">Proxy Builder</div>
						<div className="text-[10px] text-muted-foreground">
							{t("Scryfall · listo para imprimir")}
						</div>
					</div>
				</div>
				<ToggleGroup
					type="single"
					size="sm"
					data-pestanas=""
					data-tutorial="pestanas"
					className="ml-4 shrink-0"
					value={vista}
					onValueChange={(v) =>
						(v === "deck" || v === "disenar" || v === "biblioteca" || v === "mesa") &&
						cambiarVista(v)
					}
				>
					<ToggleGroupItem value="deck" className="gap-1.5 px-3 text-xs">
						<Layers className="size-3.5" />
						{t("Deck")}
					</ToggleGroupItem>
					<ToggleGroupItem value="disenar" className="gap-1.5 px-3 text-xs">
						<Brush className="size-3.5" />
						{t("Diseñar")}
					</ToggleGroupItem>
					<ToggleGroupItem value="biblioteca" className="gap-1.5 px-3 text-xs">
						<BookOpen className="size-3.5" />
						{t("Biblioteca")}
					</ToggleGroupItem>
					<ToggleGroupItem value="mesa" className="gap-1.5 px-3 text-xs">
						<Swords className="size-3.5" />
						{t("Jugar")}
						<span className="rounded-full bg-primary/15 px-1.5 text-[9px] font-semibold uppercase leading-4 text-primary">
							{t("Beta")}
						</span>
					</ToggleGroupItem>
				</ToggleGroup>
				{vista !== "biblioteca" && vista !== "mesa" && <NombreDeck />}
				<div className="ml-auto flex items-center gap-2.5">
					<SaveIndicator />
					<BotonActualizar />
					<WithTooltip content={t("Tutorial de esta pantalla")}>
						<Button
							size="icon-sm"
							variant="ghost"
							data-tutorial="ayuda"
							onClick={() => useUi.getState().abrirTutorial(vista)}
							aria-label={t("Tutorial de esta pantalla")}
						>
							<CircleHelp className="size-4" />
						</Button>
					</WithTooltip>
					<WithTooltip content={avisoAjustes}>
						<Button
							size="sm"
							variant="ghost"
							data-tutorial="ajustes"
							onClick={() => setAjustes(true)}
							aria-label={avisoAjustes ? `${t("Ajustes")}: ${avisoAjustes}` : t("Ajustes")}
							className="relative"
						>
							<Settings className="size-4" />
							{t("Ajustes")}
							<PuntoActualizacion />
						</Button>
					</WithTooltip>
				</div>
			</header>

			<main ref={principal} className="min-h-0 flex-1">
				{vista === "deck" ? (
					<ResizablePanelGroup
						orientation="horizontal"
						defaultLayout={layout.defaultLayout}
						onLayoutChanged={layout.onLayoutChanged}
					>
						<ResizablePanel id="izquierda" defaultSize="22" minSize={260}>
							{izquierda}
						</ResizablePanel>
						<ResizableHandle withHandle />
						<ResizablePanel id="centro" minSize={360}>
							{centro}
						</ResizablePanel>
						<ResizableHandle withHandle />
						<ResizablePanel id="derecha" defaultSize="22" minSize={260}>
							{derecha}
						</ResizablePanel>
					</ResizablePanelGroup>
				) : vista === "disenar" ? (
					disenar
				) : vista === "mesa" ? (
					mesa
				) : (
					biblioteca
				)}
			</main>

			<SettingsDialog open={ajustes} onOpenChange={setAjustes} />
			<Tutorial />
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
		</div>
	);
}

/** El nombre del deck abierto, editable en la barra (se guarda con el deck). */
function NombreDeck() {
	const { t } = useLang();
	const nombre = useDeck((s) => s.deck?.name ?? "");
	const [texto, setTexto] = useState(nombre);
	useEffect(() => setTexto(nombre), [nombre]);
	const confirmar = () => {
		const limpio = texto.trim();
		if (!limpio) return setTexto(nombre);
		if (limpio !== nombre) useDeck.getState().setSinHistorial({ name: limpio });
	};
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
			className="ml-2 h-7 w-64 border-transparent bg-transparent text-sm font-medium shadow-none hover:border-input focus-visible:border-input"
		/>
	);
}
