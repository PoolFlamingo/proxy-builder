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

/** Con el dedo o el lápiz no hay «encima»: el globo se abre y se cierra con un toque. */
const conRaton = (e: { pointerType?: string }) => e.pointerType === "mouse";

/**
 * La franja de incidencias de la carta, siempre en su sitio: un contador por
 * nivel (errores, avisos, información), aunque sea 0, para que la vista
 * previa no salte cuando aparece o desaparece uno. Al pasar el ratón sobre un
 * contador sale el globo con lo que hay (o que no hay nada); con el dedo, un
 * toque lo abre y otro (o tocar fuera) lo cierra.
 *
 * `compacta` (junto a la carta, en el móvil): en columna y solo los niveles con
 * algo, sin franja, para que la carta se quede con ese alto.
 */
export function Incidencias({
	avisos,
	setLang,
	compacta = false,
}: {
	avisos: Aviso[];
	setLang: (l: DesignLang) => void;
	compacta?: boolean;
}) {
	if (compacta)
		return (
			<div className="flex flex-col items-end" data-tutorial="disenar-incidencias">
				{NIVELES.map((n) => {
					const deEste = avisos.filter((a) => nivelAviso(a) === n.nivel);
					if (deEste.length === 0) return null;
					return <Contador key={n.nivel} {...n} avisos={deEste} setLang={setLang} lado="bottom" />;
				})}
			</div>
		);
	return (
		<div
			className="flex h-8 shrink-0 items-center gap-1 border-t px-2 [[data-tactil]_&]:h-11"
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
	lado = "top",
}: (typeof NIVELES)[number] & {
	avisos: Aviso[];
	setLang: (l: DesignLang) => void;
	lado?: "top" | "bottom";
}) {
	const { t } = useLang();
	const [abierto, setAbierto] = useState(false);
	const cierre = useRef<number | undefined>(undefined);
	const boton = useRef<HTMLButtonElement>(null);
	// Cómo se toca de verdad (no lo que diga la plataforma): un portátil con
	// pantalla táctil tiene las dos cosas.
	const conDedo = useRef(false);
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
					ref={boton}
					type="button"
					className={cn(
						"flex h-6 items-center gap-1 rounded-md px-1.5 text-[11px] font-medium tabular-nums outline-none hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50 [[data-tactil]_&]:h-11 [[data-tactil]_&]:min-w-11 [[data-tactil]_&]:justify-center",
						avisos.length > 0 ? color : "text-muted-foreground/60",
					)}
					aria-label={`${t(titulo)}: ${avisos.length}`}
					onPointerDown={(e) => {
						conDedo.current = !conRaton(e);
					}}
					onKeyDown={() => {
						conDedo.current = false;
					}}
					// táctil: con el dedo no hay «encima»: un toque (`onClick`) abre y cierra el
					// globo, y tocar fuera lo cierra; el entrar y salir solo valen con el ratón.
					onPointerEnter={(e) => conRaton(e) && abrir()}
					onPointerLeave={(e) => conRaton(e) && cerrar()}
					onClick={() => {
						if (conDedo.current) setAbierto((a) => !a);
					}}
					onFocus={(e) => {
						// El foco que deja un toque no abre nada: solo el del teclado.
						if (!conDedo.current && e.currentTarget.matches(":focus-visible")) abrir();
					}}
					onBlur={() => {
						if (!conDedo.current) cerrar();
					}}
				>
					<Icono icon={icono} className="size-3.5" />
					{avisos.length}
				</button>
			</PopoverAnchor>
			<PopoverContent
				side={lado}
				align={lado === "top" ? "start" : "end"}
				className="w-[min(20rem,calc(100vw-1rem))] p-0"
				// táctil: lo mismo que el botón: el globo se queda abierto mientras se lee y
				// se cierra tocando fuera (salvo en el propio botón, que ya lo alterna).
				onPointerEnter={(e) => conRaton(e) && abrir()}
				onPointerLeave={(e) => conRaton(e) && cerrar()}
				onInteractOutside={(e) => {
					if (e.target instanceof Node && boton.current?.contains(e.target)) e.preventDefault();
				}}
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
