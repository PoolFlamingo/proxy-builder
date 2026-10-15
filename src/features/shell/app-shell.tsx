import { BookOpen, Brush, Layers, Settings } from "lucide-react";
import { type ReactNode, useEffect, useState } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { SaveIndicator } from "./save-indicator";

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
}: {
	izquierda: ReactNode;
	centro: ReactNode;
	derecha: ReactNode;
	biblioteca: ReactNode;
	disenar: ReactNode;
}) {
	const { t } = useLang();
	const vista = useUi((s) => s.vista);
	const setVista = useUi((s) => s.setVista);
	const editor = useUi((s) => s.editorCartas);
	const [ajustes, setAjustes] = useState(false);
	const layout = useDefaultLayout({ id: "proxybuilder.paneles", storage: localStorage });

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
					className="ml-4 shrink-0"
					value={vista}
					onValueChange={(v) =>
						(v === "deck" || (v === "disenar" && editor) || v === "biblioteca") && setVista(v)
					}
				>
					<ToggleGroupItem value="deck" className="gap-1.5 px-3 text-xs">
						<Layers className="size-3.5" />
						{t("Deck")}
					</ToggleGroupItem>
					{editor && (
						<ToggleGroupItem value="disenar" className="gap-1.5 px-3 text-xs">
							<Brush className="size-3.5" />
							{t("Diseñar")}
							<span
								className={cn(
									"rounded-full border px-1.5 text-[9px] leading-4 font-medium uppercase",
									// En la pestaña activa (fondo de color) va del color del texto.
									vista === "disenar"
										? "border-current/50"
										: "border-amber-500/50 text-amber-600 dark:text-amber-400",
								)}
							>
								{t("Beta")}
							</span>
						</ToggleGroupItem>
					)}
					<ToggleGroupItem value="biblioteca" className="gap-1.5 px-3 text-xs">
						<BookOpen className="size-3.5" />
						{t("Biblioteca")}
					</ToggleGroupItem>
				</ToggleGroup>
				{vista !== "biblioteca" && <NombreDeck />}
				<div className="ml-auto flex items-center gap-2.5">
					<SaveIndicator />
					<Button
						size="sm"
						variant="ghost"
						onClick={() => setAjustes(true)}
						aria-label={t("Ajustes")}
					>
						<Settings className="size-4" />
						{t("Ajustes")}
					</Button>
				</div>
			</header>

			<main className="min-h-0 flex-1">
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
				) : vista === "disenar" && editor ? (
					disenar
				) : (
					biblioteca
				)}
			</main>

			<SettingsDialog open={ajustes} onOpenChange={setAjustes} />
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
			className="ml-2 h-7 w-64 border-transparent bg-transparent text-sm font-medium shadow-none hover:border-input focus-visible:border-input"
		/>
	);
}
