import { FolderOpen } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { DesignLang } from "@/bindings/DesignLang";
import { Fila } from "@/components/campos";
import { Bandera } from "@/components/emoji";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
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
import { useEstadoParaCerrar } from "@/lib/dialogos-abiertos";
import { IDIOMAS } from "@/lib/idiomas";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { completarIdioma, rehacerMiniaturas } from "./renumerar";

/**
 * **Exportar imágenes**: las cartas diseñadas como PNG, a la medida exacta
 * del corte y a 300, 600 o 900 PPP, cada idioma elegido en su carpeta
 * (`es/`, `fr/`…) de la carpeta elegida. Las copias de una carta salen una
 * vez. A un idioma que les falte a las cartas, antes se completa.
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
	const [idiomas, setIdiomas] = useState<DesignLang[]>([langDeck]);
	const [progreso, setProgreso] = useState<{ hechas: number; total: number } | null>(null);
	const jobId = useRef<number | null>(null);

	const elegidas = useMemo(() => {
		const sel = new Set(seleccion);
		return cartas.filter((c) => c.design && (alcance === "todo" || sel.has(c.id)));
	}, [cartas, seleccion, alcance]);

	const exportar = async () => {
		const carpeta = await elegirCarpetaParaExportar();
		if (!carpeta) return;
		// En el orden de la tabla, no en el de los clics.
		const langs = IDIOMAS.map((i) => i.codigo).filter((l) => idiomas.includes(l));
		setProgreso({ hechas: 0, total: 0 });
		try {
			// Los idiomas que no son el español ni el inglés, antes completos.
			const ids = new Set(elegidas.map((c) => c.id));
			const otros = langs.filter((l) => l !== "es" && l !== "en");
			for (const l of otros) await completarIdioma(l, t, mensaje, ids);
			if (otros.length > 0) await rehacerMiniaturas();
			const listas = (useDeck.getState().deck?.cards ?? []).filter((c) => ids.has(c.id));
			const { jobId: id, fin } = await lanzarTrabajo(
				"designpng://progress",
				"designpng://done",
				() =>
					call("design_export_png", {
						cards: listas,
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
	useEstadoParaCerrar({ enCurso: enMarcha, sinGuardar: false });
	return (
		<DialogContent className="sm:max-w-md" ocupado={enMarcha} alInterrumpir={cancelar}>
			<DialogHeader>
				<DialogTitle>{t("Exportar imágenes")}</DialogTitle>
				<DialogDescription className="text-xs">
					{t(
						"Cada carta diseñada como PNG, a la medida exacta del corte. Cada idioma va a su carpeta («es», «fr»…) dentro de la que elijas, con el nombre «número - nombre».",
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
				<div className="flex flex-col gap-1.5">
					<span className="text-xs font-medium">{t("Idiomas")}</span>
					<div className="grid grid-cols-3 gap-1 p-1">
						{IDIOMAS.map((i) => (
							<label
								key={i.codigo}
								className="flex min-w-0 items-center gap-1.5 text-xs"
								lang={i.locale}
							>
								<Checkbox
									checked={idiomas.includes(i.codigo)}
									disabled={enMarcha}
									onCheckedChange={(v) =>
										setIdiomas((ls) =>
											v === true ? [...ls, i.codigo] : ls.filter((l) => l !== i.codigo),
										)
									}
								/>
								<Bandera pais={i.pais} alt="" />
								<span className="truncate">{i.nativo}</span>
							</label>
						))}
					</div>
				</div>
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
					<Button
						size="sm"
						onClick={exportar}
						disabled={elegidas.length === 0 || idiomas.length === 0}
					>
						<FolderOpen />
						{t("Elegir carpeta y exportar")}
					</Button>
				)}
			</DialogFooter>
		</DialogContent>
	);
}
