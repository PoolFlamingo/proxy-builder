import { FolderOpen } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { DesignLang } from "@/bindings/DesignLang";
import { Fila } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { elegirCarpetaParaExportar } from "@/lib/dialogos";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";

type Idiomas = "es" | "en" | "ambos";

/**
 * **Exportar imágenes**: las cartas diseñadas como PNG, a la medida exacta
 * del corte y a 300, 600 o 900 PPP, en las carpetas `es/` y `en/` de la
 * carpeta elegida. Las copias de una carta salen una vez.
 */
export function PngDialog() {
	const abierto = useUi((s) => s.dialogo?.tipo === "exportarPng");
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{abierto && <Contenido />}
		</Dialog>
	);
}

function Contenido() {
	const { t, mensaje } = useLang();
	const cartas = useDeck((s) => s.deck?.cards ?? []);
	const langDeck = useDeck((s) => s.deck?.settings.designLang ?? "es");
	const seleccion = useUi((s) => s.seleccion);
	const [alcance, setAlcance] = useState<"todo" | "seleccion">(
		seleccion.length > 1 ? "seleccion" : "todo",
	);
	const [ppp, setPpp] = useState("600");
	const [idiomas, setIdiomas] = useState<Idiomas>(langDeck);
	const [progreso, setProgreso] = useState<{ hechas: number; total: number } | null>(null);
	const jobId = useRef<number | null>(null);

	const elegidas = useMemo(() => {
		const sel = new Set(seleccion);
		return cartas.filter((c) => c.design && (alcance === "todo" || sel.has(c.id)));
	}, [cartas, seleccion, alcance]);

	const exportar = async () => {
		const carpeta = await elegirCarpetaParaExportar();
		if (!carpeta) return;
		const langs: DesignLang[] = idiomas === "ambos" ? ["es", "en"] : [idiomas];
		setProgreso({ hechas: 0, total: 0 });
		try {
			const { jobId: id, fin } = await lanzarTrabajo(
				"designpng://progress",
				"designpng://done",
				() =>
					call("design_export_png", {
						cards: elegidas,
						folder: carpeta,
						cardSize: useDeck.getState().deck?.settings.cardSize ?? { widthMm: 63, heightMm: 88 },
						dpi: Number(ppp),
						langs,
					}),
				(p) => setProgreso({ hechas: p.done, total: p.total }),
			);
			jobId.current = id;
			const final = await fin;
			jobId.current = null;
			if (final.error) {
				if (final.error.code !== "Cancelled") toast.error(mensaje(final.error));
				setProgreso(null);
				return;
			}
			useUi.getState().anunciar(t("{n} imágenes exportadas", { n: final.files }));
			if (useUi.getState().dialogo?.tipo === "exportarPng") useUi.getState().abrir(null);
		} catch (e) {
			toast.error(mensaje(e));
			setProgreso(null);
		}
	};

	const cancelar = () => {
		if (jobId.current !== null) void call("job_cancel", { jobId: jobId.current });
	};

	const enMarcha = progreso !== null;
	return (
		<DialogContent className="sm:max-w-md" ocupado={enMarcha} alInterrumpir={cancelar}>
			<DialogHeader>
				<DialogTitle>{t("Exportar imágenes")}</DialogTitle>
				<DialogDescription className="text-xs">
					{t(
						"Cada carta diseñada como PNG, a la medida exacta del corte. Se guardan en las carpetas «es» y «en» de la carpeta que elijas, con el nombre «número - nombre».",
					)}
				</DialogDescription>
			</DialogHeader>
			<div className="flex flex-col gap-2.5">
				{seleccion.length > 1 && (
					<ToggleGroup
						type="single"
						size="sm"
						value={alcance}
						onValueChange={(v) => (v === "todo" || v === "seleccion") && setAlcance(v)}
						disabled={enMarcha}
					>
						<ToggleGroupItem value="todo" className="px-3 text-xs">
							{t("Todo el deck")}
						</ToggleGroupItem>
						<ToggleGroupItem value="seleccion" className="px-3 text-xs">
							{t("Solo las seleccionadas")}
						</ToggleGroupItem>
					</ToggleGroup>
				)}
				<Fila etiqueta={t("Calidad")}>
					<ToggleGroup type="single" size="sm" value={ppp} onValueChange={(v) => v && setPpp(v)}>
						{["300", "600", "900"].map((v) => (
							<ToggleGroupItem key={v} value={v} className="px-2.5 text-xs">
								{v}
							</ToggleGroupItem>
						))}
					</ToggleGroup>
				</Fila>
				<Fila etiqueta={t("Idioma")}>
					<ToggleGroup
						type="single"
						size="sm"
						value={idiomas}
						onValueChange={(v) => (v === "es" || v === "en" || v === "ambos") && setIdiomas(v)}
					>
						<ToggleGroupItem value="es" className="px-2.5 text-xs">
							ES
						</ToggleGroupItem>
						<ToggleGroupItem value="en" className="px-2.5 text-xs">
							EN
						</ToggleGroupItem>
						<ToggleGroupItem value="ambos" className="px-2.5 text-xs">
							{t("Los dos")}
						</ToggleGroupItem>
					</ToggleGroup>
				</Fila>
				<p className="text-xs text-muted-foreground">
					{elegidas.length === 0
						? t("No hay cartas diseñadas.")
						: t("{n} cartas diseñadas", { n: elegidas.length })}
				</p>
				{progreso && <Progress value={(progreso.hechas * 100) / Math.max(1, progreso.total)} />}
			</div>
			<DialogFooter>
				{enMarcha ? (
					<Button size="sm" variant="outline" onClick={cancelar}>
						{t("Cancelar")}
					</Button>
				) : (
					<Button size="sm" onClick={exportar} disabled={elegidas.length === 0}>
						<FolderOpen />
						{t("Elegir carpeta y exportar")}
					</Button>
				)}
			</DialogFooter>
		</DialogContent>
	);
}
