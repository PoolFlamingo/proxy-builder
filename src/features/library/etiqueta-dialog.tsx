import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { Tag } from "@/bindings/Tag";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { SelectorIcono } from "@/components/selector-icono";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { normalizar } from "./organizar";
import { ChipEtiqueta, COLORES_DE_ETIQUETA, COLORES_ETIQUETA } from "./piezas";
import { useBiblioteca } from "./store";

/** Qué etiqueta se crea o se edita (y a qué decks se pone al crearla). */
export type EditandoEtiqueta = { tipo: "nueva"; para: string[] } | { tipo: "editar"; tag: Tag };

/** En qué orden se reparten los colores a las etiquetas nuevas (que se distingan). */
const REPARTO = ["blue", "green", "red", "violet", "amber", "pink", "teal", "orange", "sky"];

/** El siguiente color sin usar (para que las nuevas no salgan todas iguales). */
function colorLibre(etiquetas: Tag[]): string {
	const usados = new Set(etiquetas.map((t) => t.color));
	return REPARTO.find((c) => !usados.has(c)) ?? REPARTO[etiquetas.length % REPARTO.length];
}

export function EtiquetaDialog({
	editando,
	onCerrar,
}: {
	editando: EditandoEtiqueta | null;
	onCerrar: () => void;
}) {
	const { t, mensaje } = useLang();
	const etiquetas = useBiblioteca((s) => s.etiquetas);
	const [nombre, setNombre] = useState("");
	const [color, setColor] = useState("gray");
	const [icono, setIcono] = useState<string | null>(null);
	useEffect(() => {
		if (!editando) return;
		if (editando.tipo === "editar") {
			setNombre(editando.tag.name);
			setColor(editando.tag.color);
			setIcono(editando.tag.icon);
		} else {
			setNombre("");
			setColor(colorLibre(useBiblioteca.getState().etiquetas));
			setIcono(null);
		}
	}, [editando]);
	const limpio = nombre.trim();
	const repetida = etiquetas.find(
		(x) =>
			normalizar(x.name) === normalizar(limpio) &&
			(editando?.tipo !== "editar" || x.id !== editando.tag.id),
	);
	// Crear una que ya existe es ponérsela a los decks: vale. Renombrar a otra, no.
	const bloqueada = !limpio || (editando?.tipo === "editar" && !!repetida);
	const hecho = async () => {
		if (!editando || bloqueada) return;
		const b = useBiblioteca.getState();
		try {
			if (editando.tipo === "editar") {
				await b.editarEtiqueta(editando.tag.id, limpio, color, icono);
			} else {
				const tag = await b.crearEtiqueta(limpio, color, icono);
				if (editando.para.length > 0) await b.etiquetar(editando.para, [tag.id], []);
			}
			onCerrar();
		} catch (e) {
			toast.error(mensaje(e));
		}
	};
	const vista: Tag = { id: "", name: limpio || t("Etiqueta"), color, icon: icono };
	return (
		<Dialog open={editando !== null} onOpenChange={(o) => !o && onCerrar()}>
			<DialogContent className="sm:max-w-sm" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>
						{editando?.tipo === "editar" ? t("Editar etiqueta") : t("Nueva etiqueta")}
					</DialogTitle>
					{editando?.tipo === "nueva" && editando.para.length > 0 && (
						<DialogDescription>
							{editando.para.length === 1
								? t("Se pondrá al deck elegido.")
								: t("Se pondrá a los {n} decks elegidos.", { n: editando.para.length })}
						</DialogDescription>
					)}
				</DialogHeader>
				<div className="flex flex-col gap-2.5">
					<div className="flex gap-2.5">
						<SelectorIcono valor={icono} onChange={setIcono} quitable />
						<Input
							autoFocus
							value={nombre}
							maxLength={40}
							onChange={(e) => setNombre(e.target.value)}
							onKeyDown={(e) => e.key === "Enter" && void hecho()}
							placeholder={t("Commander, Español, Por imprimir…")}
							className="text-sm"
						/>
					</div>
					<div className="flex flex-wrap gap-1.5">
						{COLORES_DE_ETIQUETA.map((c) => (
							<WithTooltip key={c} content={t(COLORES_ETIQUETA[c].nombre)}>
								<button
									type="button"
									onClick={() => setColor(c)}
									className={cn(
										"flex size-6 items-center justify-center rounded-full ring-offset-2 ring-offset-background",
										COLORES_ETIQUETA[c].punto,
										color === c && "ring-2 ring-foreground/60",
									)}
									aria-label={t(COLORES_ETIQUETA[c].nombre)}
								>
									{color === c && (
										<Icono icon="lucide:check" className="size-3.5 text-white drop-shadow" />
									)}
								</button>
							</WithTooltip>
						))}
					</div>
					<div className="flex items-center gap-2 text-[11px] text-muted-foreground">
						{t("Así se verá:")} <ChipEtiqueta tag={vista} />
					</div>
					{repetida && (
						<p className="text-[11px] text-amber-700 dark:text-amber-400">
							{editando?.tipo === "editar"
								? t("Ya hay una etiqueta «{n}».", { n: repetida.name })
								: t("Ya existe «{n}»: se usará esa.", { n: repetida.name })}
						</p>
					)}
				</div>
				<DialogFooter>
					<Button size="sm" variant="outline" onClick={onCerrar}>
						{t("Cancelar")}
					</Button>
					<Button size="sm" onClick={() => void hecho()} disabled={bloqueada}>
						{editando?.tipo === "editar" ? t("Guardar") : t("Crear")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
