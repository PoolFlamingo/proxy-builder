import { toast } from "sonner";
import type { FormatoLista } from "@/bindings/FormatoLista";
import { Icono } from "@/components/icono";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { guardarParaExportar } from "@/lib/dialogos";
import { call } from "@/lib/ipc";
import { guardarAhora, useDeck } from "@/stores/deck-store";
import { megas, nombreSeguro } from "./acciones";

/** Los formatos para copiar, como en «Export» de Moxfield. */
const COPIAR: [FormatoLista, string][] = [
	["moxfield", paraTraducir("Para Moxfield")],
	["arena", paraTraducir("Para Arena")],
	["mtgo", paraTraducir("Para MTGO")],
	["texto", paraTraducir("En texto plano")],
];

/**
 * **Otras formas de exportar** (los tres puntos junto a «Exportar PDF»): la
 * lista del deck copiada o en un `.txt` para otras webs y clientes, y el deck
 * entero con sus imágenes (`.pbdeck`). Rust escribe la lista
 * (`proxy_core::decklist::exportar`).
 */
export function MenuExportar({ deshabilitado }: { deshabilitado: boolean }) {
	const { t, mensaje } = useLang();

	const copiar = async (formato: FormatoLista, nombre: string) => {
		const deck = useDeck.getState().deck;
		if (!deck) return;
		try {
			const texto = await call("deck_list_text", { deck, format: formato });
			if (!texto.trim()) {
				toast.info(t("El deck no tiene cartas que poner en la lista."));
				return;
			}
			await navigator.clipboard.writeText(texto);
			toast.success(t("Lista copiada ({formato})", { formato: t(nombre) }));
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	const descargarMtgo = async () => {
		const deck = useDeck.getState().deck;
		if (!deck) return;
		try {
			const destino = await guardarParaExportar(
				{ title: t("Descargar para MTGO"), filters: [{ name: "MTGO", extensions: ["txt"] }] },
				nombreSeguro(deck.name, "txt"),
			);
			if (!destino) return;
			await call("deck_list_save", { deck, format: "mtgo", path: destino });
			toast.success(t("Lista guardada para MTGO"), {
				action: {
					label: t("Mostrar en carpeta"),
					onClick: () => void call("export_reveal", { path: destino }),
				},
			});
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	const paquete = async () => {
		const deck = useDeck.getState().deck;
		if (!deck) return;
		try {
			await guardarAhora();
			const destino = await guardarParaExportar(
				{
					title: t("Exportar deck con sus imágenes"),
					filters: [{ name: "Proxy Builder", extensions: ["pbdeck"] }],
				},
				nombreSeguro(deck.name, "pbdeck"),
			);
			if (!destino) return;
			const bytes = await call("deck_export_package", { id: deck.id, path: destino });
			toast.success(t("Deck exportado ({mb})", { mb: megas(bytes) }), {
				action: {
					label: t("Mostrar en carpeta"),
					onClick: () => void call("export_reveal", { path: destino }),
				},
			});
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	return (
		<DropdownMenu>
			<WithTooltip content={t("Más formas de exportar")}>
				<DropdownMenuTrigger asChild>
					<Button
						size="icon"
						disabled={deshabilitado}
						aria-label={t("Más formas de exportar")}
						className="h-auto w-8 shrink-0 rounded-l-none border-l border-primary-foreground/25"
					>
						<Icono icon="lucide:ellipsis-vertical" />
					</Button>
				</DropdownMenuTrigger>
			</WithTooltip>
			<DropdownMenuContent align="end" className="w-60 text-xs">
				<DropdownMenuLabel className="text-[10px] text-muted-foreground uppercase">
					{t("Copiar la lista")}
				</DropdownMenuLabel>
				{COPIAR.map(([formato, nombre]) => (
					<DropdownMenuItem key={formato} onSelect={() => void copiar(formato, nombre)}>
						<Icono icon="lucide:clipboard-copy" />
						{t(nombre)}
					</DropdownMenuItem>
				))}
				<DropdownMenuSeparator />
				<DropdownMenuItem onSelect={() => void descargarMtgo()}>
					<Icono icon="lucide:download" />
					{t("Descargar para MTGO (.txt)")}
				</DropdownMenuItem>
				<DropdownMenuItem onSelect={() => void paquete()}>
					<Icono icon="lucide:package" />
					{t("Deck con sus imágenes (.pbdeck)")}
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
