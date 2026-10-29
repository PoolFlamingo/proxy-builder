import { useEffect, useRef, useState } from "react";
import type { Aviso } from "@/bindings/Aviso";
import type { DesignLang } from "@/bindings/DesignLang";
import { Icono } from "@/components/icono";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Popover, PopoverAnchor, PopoverContent } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { type NivelAviso, nivelAviso, textoAviso } from "./textos";

/** Lo que cambia de un nivel a otro: icono, color y textos. */
const NIVELES: {
	nivel: NivelAviso;
	icono: string;
	color: string;
	titulo: string;
	vacio: string;
}[] = [
	{
		nivel: "error",
		icono: "lucide:octagon-alert",
		color: "text-destructive",
		titulo: paraTraducir("Errores"),
		vacio: paraTraducir("No hay ningún error que mostrar."),
	},
	{
		nivel: "aviso",
		icono: "lucide:triangle-alert",
		color: "text-amber-700 dark:text-amber-400",
		titulo: paraTraducir("Avisos"),
		vacio: paraTraducir("No hay ningún aviso que mostrar."),
	},
	{
		nivel: "info",
		icono: "lucide:info",
		color: "text-muted-foreground",
		titulo: paraTraducir("Información"),
		vacio: paraTraducir("No hay ninguna información que mostrar."),
	},
];

/** Lo que tarda en cerrarse al salir el ratón (para poder pasar al globo). */
const CIERRE_MS = 150;

/**
 * La franja de incidencias de la carta, siempre en su sitio: un contador por
 * nivel (errores, avisos, información), aunque sea 0, para que la vista
 * previa no salte cuando aparece o desaparece uno. Al pasar el ratón sobre un
 * contador sale el globo con lo que hay (o que no hay nada).
 */
export function Incidencias({
	avisos,
	setLang,
}: {
	avisos: Aviso[];
	setLang: (l: DesignLang) => void;
}) {
	return (
		<div
			className="flex h-8 shrink-0 items-center gap-1 border-t px-2"
			data-tutorial="disenar-incidencias"
		>
			{NIVELES.map((n) => (
				<Contador
					key={n.nivel}
					{...n}
					avisos={avisos.filter((a) => nivelAviso(a) === n.nivel)}
					setLang={setLang}
				/>
			))}
		</div>
	);
}

function Contador({
	icono,
	color,
	titulo,
	vacio,
	avisos,
	setLang,
}: (typeof NIVELES)[number] & { avisos: Aviso[]; setLang: (l: DesignLang) => void }) {
	const { t } = useLang();
	const [abierto, setAbierto] = useState(false);
	const cierre = useRef<number | undefined>(undefined);
	const abrir = () => {
		window.clearTimeout(cierre.current);
		setAbierto(true);
	};
	const cerrar = () => {
		window.clearTimeout(cierre.current);
		cierre.current = window.setTimeout(() => setAbierto(false), CIERRE_MS);
	};
	useEffect(() => () => window.clearTimeout(cierre.current), []);

	return (
		<Popover open={abierto} onOpenChange={setAbierto}>
			<PopoverAnchor asChild>
				<button
					type="button"
					className={cn(
						"flex h-6 items-center gap-1 rounded-md px-1.5 text-[11px] font-medium tabular-nums outline-none hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50",
						avisos.length > 0 ? color : "text-muted-foreground/60",
					)}
					aria-label={`${t(titulo)}: ${avisos.length}`}
					onPointerEnter={abrir}
					onPointerLeave={cerrar}
					onFocus={abrir}
					onBlur={cerrar}
				>
					<Icono icon={icono} className="size-3.5" />
					{avisos.length}
				</button>
			</PopoverAnchor>
			<PopoverContent
				side="top"
				align="start"
				className="w-80 p-0"
				onPointerEnter={abrir}
				onPointerLeave={cerrar}
				onOpenAutoFocus={(e) => e.preventDefault()}
				onCloseAutoFocus={(e) => e.preventDefault()}
			>
				<p className="border-b px-2.5 py-1.5 text-xs font-semibold">{t(titulo)}</p>
				{avisos.length === 0 ? (
					<p className="px-2.5 py-2 text-[11px] text-muted-foreground">{t(vacio)}</p>
				) : (
					<ScrollArea viewportClassName="max-h-64">
						<ul className="flex flex-col gap-2 p-2.5">
							{avisos.map((a) => (
								<li
									key={JSON.stringify(a)}
									className="flex items-start gap-1.5 text-[11px] text-popover-foreground"
								>
									<Icono icon={icono} className={cn("mt-px size-3.5 shrink-0", color)} />
									<span className="min-w-0 flex-1">{textoAviso(a, t)}</span>
									{a.kind === "faltanLetras" && (
										<Button size="xs" variant="outline" onClick={() => setLang(a.lang)}>
											<Icono icon="lucide:download" />
											{t("Bajar las letras")}
										</Button>
									)}
								</li>
							))}
						</ul>
					</ScrollArea>
				)}
			</PopoverContent>
		</Popover>
	);
}
