import { useEffect, useMemo, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import type { IdJugador } from "@/bindings/IdJugador";
import type { Senal } from "@/bindings/Senal";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { localeDe } from "@/lib/idiomas";
import { cn } from "@/lib/utils";
import { useChat } from "@/stores/chat-store";
import { tableroDe, useMesa } from "@/stores/mesa-store";
import { useRepeticion } from "@/stores/repeticion-store";
import { hacer } from "./acciones";
import { ATAJOS } from "./atajos";
import {
	AVISOS,
	MAX_CHAT,
	mensajesDeChat,
	SENALES,
	teclaDeComunicacion,
	textoDeAviso,
} from "./cuentas-chat";
import { usePantalla } from "./pantalla";
import { useDisposicion } from "./selector-disposicion";

/**
 * **El chat de la mesa** (H5): mensajes de hasta 500 caracteres y avisos de
 * un clic (Espera, Respondo, OK…). Todo es `Accion::Chat` / `Accion::Aviso`
 * del motor, así que sale en el registro y en la repetición; silenciar a
 * alguien lo oculta solo para mí (`chat-store`).
 */
export function ChatPanel() {
	const { t, lang } = useLang();
	const registro = useMesa((s) => s.registro);
	const jugadores = useMesa((s) => s.jugadores);
	const asientos = useMesa(useShallow((s) => s.asientos));
	const yo = useMesa((s) => s.yo);
	const silenciados = useChat(useShallow((s) => s.silenciados));
	const alternar = useChat((s) => s.alternarSilencio);
	const repitiendo = useRepeticion((s) => s.activa !== null);
	const [texto, setTexto] = useState("");
	const final = useRef<HTMLDivElement>(null);
	const campo = useRef<HTMLInputElement>(null);
	const mensajes = useMemo(() => mensajesDeChat(registro, silenciados), [registro, silenciados]);
	const hora = useMemo(
		() => new Intl.DateTimeFormat(localeDe(lang), { hour: "2-digit", minute: "2-digit" }),
		[lang],
	);
	const puedeEscribir = yo !== null && !repitiendo;

	// Al abrir el panel (o con Enter) el cursor va al campo; lo nuevo, al final.
	useEffect(() => {
		// Tras el clic en la pestaña, que se queda con el foco si no se espera.
		const f = requestAnimationFrame(() => campo.current?.focus());
		useChat.getState().marcarLeido();
		return () => cancelAnimationFrame(f);
	}, []);
	useEffect(() => {
		final.current?.scrollIntoView({ block: "end" });
	}, [mensajes.length]);

	const enviar = () => {
		const limpio = texto.trim();
		if (!limpio || !puedeEscribir) return;
		void hacer({ que: "chat", texto: limpio }).then((ok) => ok && setTexto(""));
	};

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-2">
			{asientos.length > 1 && (
				<div className="flex flex-wrap gap-1 px-1">
					{asientos
						.filter((id) => id !== yo)
						.map((id) => {
							const callado = silenciados.includes(id);
							return (
								<WithTooltip
									key={id}
									content={callado ? t("Dejar de silenciar") : t("Silenciar (solo para mí)")}
								>
									<Button
										size="xs"
										variant={callado ? "secondary" : "ghost"}
										className="gap-1"
										onClick={() => alternar(id)}
										aria-pressed={callado}
									>
										<span
											className="size-2 rounded-full"
											style={{ background: jugadores[id]?.color }}
										/>
										{jugadores[id]?.nombre ?? "?"}
										{callado ? (
											<Icono icon="lucide:mic-off" className="size-3" />
										) : (
											<Icono icon="lucide:mic" className="size-3" />
										)}
									</Button>
								</WithTooltip>
							);
						})}
				</div>
			)}
			<ScrollArea className="min-h-0 flex-1" viewportClassName="h-full">
				<div className="flex flex-col gap-1.5 p-1 text-xs">
					{mensajes.map((m) => (
						<div key={m.seq} className="flex flex-col rounded-lg bg-muted/40 px-2 py-1">
							<span className="flex items-center gap-1.5 text-[10px] text-muted-foreground">
								<span
									className="size-2 rounded-full"
									style={{ background: jugadores[m.jugador]?.color }}
								/>
								<b className="font-medium text-foreground">{jugadores[m.jugador]?.nombre ?? "?"}</b>
								<span className="tabular-nums">{hora.format(new Date(m.tMs))}</span>
							</span>
							{m.texto !== null ? (
								<span className="break-words whitespace-pre-wrap">{m.texto}</span>
							) : (
								<span className="text-primary italic">{t(textoDeAviso(m.aviso ?? "ok"))}</span>
							)}
						</div>
					))}
					{mensajes.length === 0 && (
						<p className="p-4 text-center text-muted-foreground">{t("Todavía no hay mensajes.")}</p>
					)}
					<div ref={final} />
				</div>
			</ScrollArea>
			{puedeEscribir ? (
				<div className="flex flex-col gap-1.5">
					<div className="flex flex-wrap gap-1">
						{AVISOS.map((a) => (
							<WithTooltip key={a.tipo} content={a.tecla}>
								<Button
									size="xs"
									variant="outline"
									onClick={() => void hacer({ que: "aviso", tipo: a.tipo })}
								>
									{t(a.texto)}
								</Button>
							</WithTooltip>
						))}
					</div>
					<div className="flex gap-1.5">
						<Input
							ref={campo}
							value={texto}
							maxLength={MAX_CHAT}
							placeholder={t("Escribe un mensaje…")}
							aria-label={t("Escribe un mensaje…")}
							onChange={(e) => setTexto(e.target.value)}
							onKeyDown={(e) => {
								if (e.key === "Enter") {
									e.preventDefault();
									enviar();
								} else if (e.key === "Escape") {
									e.currentTarget.blur();
								}
							}}
						/>
						<Button size="icon" onClick={enviar} disabled={!texto.trim()} aria-label={t("Enviar")}>
							<Icono icon="lucide:send" className="size-4" />
						</Button>
					</div>
				</div>
			) : (
				<p className="px-1 text-center text-[11px] text-muted-foreground">
					{repitiendo
						? t("En la repetición no se escribe.")
						: t("Los espectadores no pueden escribir.")}
				</p>
			)}
		</div>
	);
}

/** El bocadillo con lo último que dijo un jugador (unos segundos). */
export function Bocadillo({ jugador, className }: { jugador: IdJugador; className?: string }) {
	const { t } = useLang();
	const b = useChat((s) => s.bocadillos[jugador]);
	if (!b) return null;
	return (
		<div
			className={cn(
				"pointer-events-none z-[7600] max-w-56 rounded-xl rounded-bl-sm border border-white/15 bg-black/80 px-2.5 py-1.5 text-xs break-words text-white shadow-lg backdrop-blur",
				className,
			)}
		>
			{b.texto !== null ? b.texto : t(textoDeAviso(b.aviso ?? "ok"))}
		</div>
	);
}

/**
 * Los bocadillos de quienes no tienen su miniatura a la vista (la del tablero
 * grande y todos si se ve «un solo tablero»): una pila arriba a la izquierda.
 */
export function BocadillosFlotantes() {
	const { config } = useDisposicion();
	const asientos = useMesa(useShallow((s) => s.asientos));
	const grande = useMesa(tableroDe);
	const sinMini = asientos.filter((id) => config.modo === "unico" || id === grande);
	return (
		<div className="pointer-events-none absolute top-14 left-3 z-[7600] flex flex-col gap-1.5">
			{sinMini.map((id) => (
				<BocadilloConNombre key={id} jugador={id} />
			))}
		</div>
	);
}

function BocadilloConNombre({ jugador }: { jugador: IdJugador }) {
	const habla = useChat((s) => s.bocadillos[jugador] !== undefined);
	const j = useMesa((s) => s.jugadores[jugador]);
	if (!habla) return null;
	return (
		<div className="flex items-end gap-1.5">
			<span
				className="mb-1 rounded px-1.5 py-0.5 text-[10px] font-medium text-white"
				style={{ background: j?.color }}
			>
				{j?.nombre}
			</span>
			<Bocadillo jugador={jugador} />
		</div>
	);
}

const ICONOS_SENAL: Record<Senal, string> = {
	sinRespuesta: "lucide:circle-check",
	espera: "lucide:hand",
	pensando: "lucide:brain",
	ausente: "lucide:user-x",
};

/** La señal de prioridad de un jugador, junto a su nombre o su vida. */
export function InsigniaSenal({ jugador, className }: { jugador: IdJugador; className?: string }) {
	const { t } = useLang();
	const senal = useMesa((s) => s.jugadores[jugador]?.senal ?? null);
	if (!senal) return null;
	const icono = ICONOS_SENAL[senal];
	const texto = SENALES.find((s) => s.senal === senal)?.texto ?? "";
	return (
		<WithTooltip content={t(texto)}>
			<span
				className={cn(
					"inline-flex size-4 items-center justify-center rounded-full bg-primary text-primary-foreground",
					className,
				)}
				role="img"
				aria-label={t(texto)}
			>
				<Icono icon={icono} className="size-2.5" />
			</span>
		</WithTooltip>
	);
}

/**
 * Los atajos de avisos (Alt+1…6) y señales (Ctrl+Mayús+1…4; Enter para
 * escribir lo maneja el tablero). Van por el código físico de la tecla: con
 * Mayús o Alt la tecla de un dígito cambia de símbolo.
 */
export function useAtajosComunicacion() {
	useEffect(() => {
		const alTeclear = (e: KeyboardEvent) => {
			const el = e.target as HTMLElement | null;
			if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable)) {
				return;
			}
			if (useRepeticion.getState().activa) return;
			if (usePantalla.getState().dialogo || document.querySelector('[role="dialog"]')) return;
			const tecla = teclaDeComunicacion(e);
			if (!tecla) return;
			const atajo = ATAJOS.find((a) => a.contexto === "mesa" && a.tecla === tecla);
			if (!atajo) return;
			e.preventDefault();
			atajo.hacer(null);
		};
		window.addEventListener("keydown", alTeclear);
		return () => window.removeEventListener("keydown", alTeclear);
	}, []);
}
