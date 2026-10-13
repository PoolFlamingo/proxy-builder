import { Save, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import type { Offset } from "@/bindings/Offset";
import type { Preset } from "@/bindings/Preset";
import type { PresetKind } from "@/bindings/PresetKind";
import type { PrintSettings } from "@/bindings/PrintSettings";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";

/**
 * Guardar y cargar presets con nombre (Fase 7): los desplazamientos de fila
 * de cada impresora (frente y reverso) o los ajustes de impresión enteros.
 * Viven en la biblioteca local, no en el deck: sirven para todos.
 */
function Presets({
	kind,
	actual,
	aplicar,
}: {
	kind: PresetKind;
	actual: () => string;
	aplicar: (json: string) => void;
}) {
	const { t, mensaje } = useLang();
	const [lista, setLista] = useState<Preset[]>([]);
	const [elegido, setElegido] = useState<string>("");
	const [nombre, setNombre] = useState("");
	const [abierto, setAbierto] = useState(false);

	const refrescar = useCallback(() => {
		call("preset_list", { kind })
			.then(setLista)
			.catch((e) => toast.error(mensaje(e)));
	}, [kind, mensaje]);
	useEffect(refrescar, []);

	const cargar = (id: string) => {
		const p = lista.find((x) => x.id === id);
		if (!p) return;
		setElegido(id);
		try {
			aplicar(p.json);
			useUi.getState().anunciar(t("Preset «{n}» cargado", { n: p.name }));
		} catch {
			toast.error(t("Ese preset está dañado"));
		}
	};

	const guardar = async () => {
		try {
			const p = await call("preset_save", { kind, name: nombre.trim(), json: actual() });
			setElegido(p.id);
			setNombre("");
			setAbierto(false);
			refrescar();
			useUi.getState().anunciar(t("Preset «{n}» guardado", { n: p.name }));
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	const borrar = async () => {
		try {
			await call("preset_delete", { id: elegido });
			setElegido("");
			refrescar();
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	return (
		<div className="flex items-center gap-1.5">
			<Select value={elegido} onValueChange={cargar}>
				<SelectTrigger
					size="sm"
					className="h-7 min-w-0 flex-1 text-xs"
					disabled={lista.length === 0}
				>
					<SelectValue placeholder={lista.length ? t("Cargar preset…") : t("Sin presets")} />
				</SelectTrigger>
				<SelectContent>
					{lista.map((p) => (
						<SelectItem key={p.id} value={p.id} className="text-xs">
							{p.name}
						</SelectItem>
					))}
				</SelectContent>
			</Select>
			<Popover open={abierto} onOpenChange={setAbierto}>
				<WithTooltip content={t("Guardar como preset")}>
					<PopoverTrigger asChild>
						<Button
							size="icon-sm"
							variant="outline"
							className="size-7"
							aria-label={t("Guardar como preset")}
						>
							<Save />
						</Button>
					</PopoverTrigger>
				</WithTooltip>
				<PopoverContent className="flex w-64 flex-col gap-2 p-2.5">
					<span className="text-xs font-medium">{t("Nombre del preset")}</span>
					<Input
						autoFocus
						value={nombre}
						onChange={(e) => setNombre(e.target.value)}
						onKeyDown={(e) => e.key === "Enter" && nombre.trim() && void guardar()}
						placeholder={t("p. ej. Brother HL-L2350")}
						className="h-7 text-xs"
					/>
					<Button size="sm" disabled={!nombre.trim()} onClick={guardar}>
						{t("Guardar")}
					</Button>
				</PopoverContent>
			</Popover>
			<WithTooltip content={t("Borrar el preset elegido")}>
				<Button
					size="icon-sm"
					variant="ghost"
					className="size-7"
					disabled={!elegido}
					onClick={borrar}
					aria-label={t("Borrar el preset elegido")}
				>
					<Trash2 />
				</Button>
			</WithTooltip>
		</div>
	);
}

export function PresetsOffsets({ kind }: { kind: "frontOffsets" | "backOffsets" }) {
	const cara = kind === "frontOffsets" ? "front" : "back";
	return (
		<Presets
			kind={kind}
			actual={() => JSON.stringify(useDeck.getState().deck?.settings.rowOffsets[cara] ?? [])}
			aplicar={(json) => {
				const filas = JSON.parse(json) as Offset[];
				if (!Array.isArray(filas)) throw new Error("preset");
				useDeck
					.getState()
					.setAjustes((s) => ({ ...s, rowOffsets: { ...s.rowOffsets, [cara]: filas } }));
			}}
		/>
	);
}

export function PresetsImpresion() {
	return (
		<Presets
			kind="print"
			actual={() => JSON.stringify(useDeck.getState().deck?.settings ?? {})}
			aplicar={(json) => {
				const s = JSON.parse(json) as PrintSettings;
				if (typeof s !== "object" || !s || !("paper" in s)) throw new Error("preset");
				useDeck.getState().setAjustes((actual) => ({ ...actual, ...s }));
			}}
		/>
	);
}
