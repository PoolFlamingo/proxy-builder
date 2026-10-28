import {
	Crown,
	Dices,
	Dot,
	Flag,
	Ghost,
	Hash,
	Heart,
	Hourglass,
	Layers,
	Library,
	MessageSquare,
	Move,
	RotateCw,
	Undo2,
	X,
} from "lucide-react";
import { Fragment, useMemo } from "react";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { localeDe } from "@/lib/idiomas";
import { cn } from "@/lib/utils";
import { useChat } from "@/stores/chat-store";
import { useMesa } from "@/stores/mesa-store";
import { useRepeticion } from "@/stores/repeticion-store";
import { hacer } from "./acciones";
import { type CategoriaEvento, categoriaDeEvento } from "./categorias-eventos";
import { ChatPanel } from "./chat";
import { formatoDuracion } from "./cuentas-fin";
import { cartaParaApilar, duracionDeTurno } from "./cuentas-registro";
import { ANCHO_PANEL, type PanelMesa, usePantalla } from "./pantalla";
import { useSenalarFantasma } from "./sparring/vista-fantasma";
import { piezasDeTexto, textoDeEvento } from "./textos-eventos";

const ICONOS: Record<CategoriaEvento, React.ComponentType<{ className?: string }>> = {
	mover: Move,
	girar: RotateCw,
	contadores: Hash,
	vida: Heart,
	mando: Crown,
	biblioteca: Library,
	turno: Hourglass,
	azar: Dices,
	chat: MessageSquare,
	fin: Flag,
	sparring: Ghost,
	deshacer: Undo2,
	otro: Dot,
};

const COLORES: Record<CategoriaEvento, string> = {
	mover: "text-sky-400",
	girar: "text-violet-400",
	contadores: "text-amber-400",
	vida: "text-rose-400",
	mando: "text-orange-400",
	biblioteca: "text-emerald-400",
	turno: "text-primary",
	azar: "text-fuchsia-400",
	chat: "text-muted-foreground",
	fin: "text-primary",
	sparring: "text-purple-300",
	deshacer: "text-muted-foreground",
	otro: "text-muted-foreground",
};

/**
 * **El panel de al lado de la mesa** (H4): el registro de la partida y el
 * chat en pestañas. Se abre y se cierra con el botón de la barra; no tapa
 * los diálogos y la vista grande de las cartas se aparta de él.
 */
export function PanelLateral() {
	const { t } = useLang();
	const panel = usePantalla((s) => s.panel);
	const setPanel = usePantalla((s) => s.setPanel);
	const sinLeer = useChat((s) => s.sinLeer);
	if (!panel) return null;
	const cambiar = (p: string) => {
		setPanel(p as PanelMesa);
		if (p === "chat") useChat.getState().marcarLeido();
	};
	return (
		<aside
			className="absolute top-14 right-2 bottom-2 z-[7500] flex flex-col gap-2 rounded-xl border border-white/10 bg-card p-2 shadow-2xl"
			style={{ width: ANCHO_PANEL }}
			data-tutorial="mesa-panel"
		>
			<Tabs value={panel} onValueChange={cambiar} className="flex min-h-0 flex-1 flex-col gap-2">
				<div className="flex items-center gap-1.5">
					<TabsList className="flex-1">
						<TabsTrigger value="registro">{t("Registro")}</TabsTrigger>
						<TabsTrigger value="chat">
							{t("Chat")}
							{sinLeer > 0 && panel !== "chat" && (
								<span className="rounded-full bg-primary px-1.5 text-[10px] text-primary-foreground tabular-nums">
									{sinLeer}
								</span>
							)}
						</TabsTrigger>
					</TabsList>
					<WithTooltip content={t("Cerrar")}>
						<Button
							size="icon-sm"
							variant="ghost"
							onClick={() => setPanel(null)}
							aria-label={t("Cerrar")}
						>
							<X className="size-4" />
						</Button>
					</WithTooltip>
				</div>
				<TabsContent value="registro" className="flex min-h-0 flex-1 flex-col">
					<RegistroLista />
				</TabsContent>
				<TabsContent value="chat" className="flex min-h-0 flex-1 flex-col">
					<ChatPanel />
				</TabsContent>
			</Tabs>
		</aside>
	);
}

/**
 * **El registro de la partida**: cada evento en una frase con su icono y su
 * hora, lo último arriba. Pasar el ratón por el nombre de una carta la enseña
 * en grande; «A la pila» (las últimas jugadas al campo) y «Deshacer hasta
 * aquí» (en solitario) salen al pasar por la línea.
 */
function RegistroLista() {
	const { t, lang } = useLang();
	const registro = useMesa((s) => s.registro);
	const cartas = useMesa((s) => s.cartas);
	const jugadores = useMesa((s) => s.jugadores);
	const solitario = useMesa((s) => s.asientos.length === 1);
	const repitiendo = useRepeticion((s) => s.activa !== null);
	const hora = useMemo(
		() =>
			new Intl.DateTimeFormat(localeDe(lang), {
				hour: "2-digit",
				minute: "2-digit",
				second: "2-digit",
			}),
		[lang],
	);
	const lineas = useMemo(
		() =>
			registro.map((ev, i) => {
				const ms = duracionDeTurno(registro, i);
				return {
					ev,
					i,
					texto: textoDeEvento(
						ev,
						{ cartas, jugadores },
						t,
						true,
						ms === null ? undefined : formatoDuracion(ms),
					),
				};
			}),
		[registro, cartas, jugadores, t],
	);
	return (
		<ScrollArea className="min-h-0 flex-1" viewportClassName="h-full">
			<div className="flex flex-col gap-0.5 p-1 text-xs">
				{[...lineas].reverse().map(({ ev, i, texto }) => {
					const categoria = categoriaDeEvento(ev);
					const Icono = ICONOS[categoria];
					const apilar = repitiendo ? null : cartaParaApilar(registro, i, cartas);
					return (
						<div
							key={ev.seq}
							className="group flex items-start gap-1.5 rounded px-1.5 py-1 hover:bg-muted/60"
						>
							<Icono className={cn("mt-0.5 size-3.5 shrink-0", COLORES[categoria])} />
							<span className="min-w-0 flex-1">
								<span className="break-words">
									{piezasDeTexto(texto).map((p, k) =>
										typeof p === "string" ? (
											<Fragment key={k}>{p}</Fragment>
										) : "fantasma" in p ? (
											<NombreDeFantasma key={k} nombre={p.fantasma} />
										) : (
											<NombreDeCarta key={k} carta={p.carta} nombre={p.nombre} />
										),
									)}
								</span>
								<span className="block text-[10px] text-muted-foreground tabular-nums">
									{hora.format(new Date(Number(ev.tMs)))}
								</span>
							</span>
							<span className="flex shrink-0 items-center gap-0.5">
								{apilar !== null && (
									<Button
										size="xs"
										variant="outline"
										className="opacity-0 group-hover:opacity-100"
										onClick={() => void hacer({ que: "apilarDespues", carta: apilar })}
									>
										<Layers className="size-3" />
										{t("A la pila")}
									</Button>
								)}
								{solitario && !repitiendo && ev.cambio.tipo !== "restaurar" && (
									<WithTooltip content={t("Deshacer hasta aquí")}>
										<Button
											size="icon-sm"
											variant="ghost"
											className="opacity-0 group-hover:opacity-100"
											onClick={() => void hacer({ que: "deshacer", hasta: Number(ev.seq) })}
											aria-label={t("Deshacer hasta aquí")}
										>
											<Undo2 className="size-3.5" />
										</Button>
									</WithTooltip>
								)}
							</span>
						</div>
					);
				})}
				{registro.length === 0 && (
					<p className="p-4 text-center text-muted-foreground">{t("Todavía no ha pasado nada.")}</p>
				)}
			</div>
		</ScrollArea>
	);
}

/** El nombre de la carta del fantasma en el registro: al pasar el ratón sale en grande. */
function NombreDeFantasma({ nombre }: { nombre: string }) {
	const senalar = useSenalarFantasma(nombre);
	return (
		<span
			className="cursor-help font-medium underline decoration-dotted underline-offset-2"
			{...senalar}
		>
			{nombre}
		</span>
	);
}

/** El nombre de una carta en el registro: al pasar el ratón sale en grande. */
function NombreDeCarta({ carta, nombre }: { carta: number; nombre: string }) {
	const existe = useMesa((s) => s.cartas[carta] !== undefined);
	if (!existe) return <>{nombre}</>;
	return (
		<span
			className="cursor-help font-medium underline decoration-dotted underline-offset-2"
			onPointerEnter={() => useMesa.getState().setSobre(carta)}
			onPointerLeave={() => {
				if (useMesa.getState().sobre === carta) useMesa.getState().setSobre(null);
			}}
		>
			{nombre}
		</span>
	);
}
